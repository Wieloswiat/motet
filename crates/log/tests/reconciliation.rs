//! An uncertain write can be acknowledged only if its exact bytes are durable.

use std::fmt;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures::stream::{self, BoxStream};
use motet_log::{AppendError, Log, Reconciled, Writer};
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::{
    CopyOptions, Error, GetOptions, GetResult, GetResultPayload, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
    PutMultipartOptions, PutOptions, PutPayload, PutResult, Result,
};
use proptest::prelude::*;

#[derive(Clone, Copy, Debug)]
enum Fault {
    BeforePut,
    AfterPut,
    BeforeGet,
    DuringGet,
}

/// Inject a transport fault without replacing the store's conditional writes.
#[derive(Debug, Default)]
struct FaultStore {
    inner: InMemory,
    fault: Mutex<Option<Fault>>,
}

impl FaultStore {
    fn set(&self, fault: Option<Fault>) {
        *self.fault.lock().expect("fault lock is not poisoned") = fault;
    }

    fn take_if(&self, applies: impl FnOnce(Fault) -> bool) -> Option<Fault> {
        let mut fault = self.fault.lock().expect("fault lock is not poisoned");
        if fault.is_some_and(applies) { fault.take() } else { None }
    }
}

impl fmt::Display for FaultStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("fault-injecting memory store")
    }
}

fn timeout() -> Error {
    Error::Generic {
        store: "fault-injecting memory store",
        source: std::io::Error::from(std::io::ErrorKind::TimedOut).into(),
    }
}

#[async_trait::async_trait]
impl ObjectStore for FaultStore {
    async fn put_opts(&self, path: &Path, payload: PutPayload, opts: PutOptions) -> Result<PutResult> {
        let fault = self.take_if(|fault| matches!(fault, Fault::BeforePut | Fault::AfterPut));
        if matches!(fault, Some(Fault::BeforePut)) {
            return Err(timeout());
        }
        let result = self.inner.put_opts(path, payload, opts).await?;
        if matches!(fault, Some(Fault::AfterPut)) {
            return Err(timeout());
        }
        Ok(result)
    }

    async fn get_opts(&self, path: &Path, opts: GetOptions) -> Result<GetResult> {
        let fault = self.take_if(|fault| matches!(fault, Fault::BeforeGet | Fault::DuringGet));
        if matches!(fault, Some(Fault::BeforeGet)) {
            return Err(timeout());
        }
        let mut result = self.inner.get_opts(path, opts).await?;
        if matches!(fault, Some(Fault::DuringGet)) {
            result.payload = GetResultPayload::Stream(Box::pin(stream::once(async { Err(timeout()) })));
        }
        Ok(result)
    }

    async fn put_multipart_opts(&self, path: &Path, opts: PutMultipartOptions) -> Result<Box<dyn MultipartUpload>> {
        self.inner.put_multipart_opts(path, opts).await
    }

    fn delete_stream(&self, paths: BoxStream<'static, Result<Path>>) -> BoxStream<'static, Result<Path>> {
        self.inner.delete_stream(paths)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(&self, from: &Path, to: &Path, opts: CopyOptions) -> Result<()> {
        self.inner.copy_opts(from, to, opts).await
    }
}

#[derive(Clone, Copy, Debug)]
enum Rival {
    None,
    OpenBeforePut,
    OpenAfterPut,
}

#[derive(Debug)]
struct Round {
    stored: bool,
    rival: Rival,
    failed_reads: Vec<bool>,
    cursor: u64,
}

fn round() -> impl Strategy<Value = Round> {
    (
        any::<bool>(),
        prop_oneof![Just(Rival::None), Just(Rival::OpenBeforePut), Just(Rival::OpenAfterPut)],
        prop::collection::vec(any::<bool>(), 0..4),
        prop_oneof![Just(0), 0..30_u64, Just(u64::MAX)],
    )
        .prop_map(|(stored, rival, failed_reads, cursor)| Round {
            stored,
            rival,
            failed_reads,
            cursor,
        })
}

async fn assert_replay(log: &Log, committed: &[Bytes], cursor: u64) {
    let actual = log.read_from(cursor).await.expect("contiguous replay");
    assert!(
        actual
            .iter()
            .map(|(seq, batch)| (*seq, batch))
            .eq(committed.iter().enumerate().filter_map(|(index, batch)| {
                let seq = index as u64;
                (seq >= cursor).then_some((seq, batch))
            })),
        "incorrect replay from {cursor}; actual: {actual:?}; committed: {committed:?}"
    );
}

async fn assert_pending(
    store: &FaultStore,
    writer: &mut Writer,
    round: &Round,
    position: u64,
    batch: &Bytes,
    object_exists: bool,
) {
    for &lose_body in &round.failed_reads {
        // With no object, there is no response body to interrupt.
        let fault = if lose_body && object_exists {
            Fault::DuringGet
        } else {
            Fault::BeforeGet
        };
        store.set(Some(fault));
        let result = writer.reconcile(batch).await;
        assert!(
            matches!(result, Err(AppendError::Unknown { seq, .. }) if seq == position),
            "{round:?}: {result:?}"
        );
        store.set(None);
    }
    let result = writer.append(Bytes::from_static(b"must not be written")).await;
    assert!(
        matches!(result, Err(AppendError::Unresolved(seq)) if seq == position),
        "{round:?}: {result:?}"
    );
}

async fn check_history(rounds: &[Round]) {
    let store = Arc::new(FaultStore::default());
    let log = Log::new(Arc::<FaultStore>::clone(&store), "log");
    let mut writer = log.writer().await.expect("initial writer");
    // Only the fault schedule tells the model what reached the bucket. Neither
    // the writer's next position nor its reconciliation result is an oracle.
    let mut committed = Vec::new();
    for (number, round) in rounds.iter().enumerate() {
        let batch = Bytes::from(format!("round {number}: original"));
        let rival_batch = Bytes::from(format!("round {number}: rival"));
        let position = committed.len() as u64;
        let mut rival = match round.rival {
            Rival::OpenBeforePut => Some(log.writer().await.expect("rival opens before the PUT")),
            _ => None,
        };
        store.set(Some(if round.stored {
            Fault::AfterPut
        } else {
            Fault::BeforePut
        }));
        let result = writer.append(batch.clone()).await;
        assert!(
            matches!(result, Err(AppendError::Unknown { seq, .. }) if seq == position),
            "{round:?}: {result:?}"
        );
        if round.stored {
            committed.push(batch.clone());
        }

        if matches!(round.rival, Rival::OpenAfterPut) {
            rival = Some(log.writer().await.expect("rival opens after the PUT"));
        }
        let rival_commits = match round.rival {
            Rival::None => false,
            Rival::OpenBeforePut => !round.stored,
            Rival::OpenAfterPut => true,
        };
        if let Some(contender) = &mut rival {
            let result = contender.append(rival_batch.clone()).await;
            if rival_commits {
                assert_eq!(
                    result.expect("rival commits to an unused position"),
                    committed.len() as u64
                );
                committed.push(rival_batch);
            } else {
                assert!(
                    matches!(result, Err(AppendError::Fenced(seq)) if seq == position),
                    "{round:?}: {result:?}"
                );
            }
        }

        assert_pending(
            &store,
            &mut writer,
            round,
            position,
            &batch,
            round.stored || rival_commits,
        )
        .await;
        let result = writer.reconcile(&batch).await;
        if round.stored {
            assert_eq!(
                result.expect("matching bytes acknowledge the original PUT"),
                Reconciled::Durable(position)
            );
        } else if rival_commits {
            assert!(
                matches!(result, Err(AppendError::Fenced(seq)) if seq == position),
                "{round:?}: {result:?}"
            );
        } else {
            assert_eq!(result.expect("missing bytes permit a retry"), Reconciled::Absent);
        }

        if rival_commits {
            if round.stored {
                let result = writer.append(Bytes::from_static(b"stale next position")).await;
                assert!(
                    matches!(result, Err(AppendError::Fenced(seq)) if seq == position + 1),
                    "{round:?}: {result:?}"
                );
            }
            writer = rival.expect("the rival won");
        }
        // An absent attempt can be retried at the same position. After a durable
        // append or takeover, a different batch must go to the next position.
        let continuation = if !round.stored && !rival_commits {
            batch
        } else {
            Bytes::from(format!("round {number}: continuation"))
        };
        assert_eq!(
            writer
                .append(continuation.clone())
                .await
                .expect("live writer can continue"),
            committed.len() as u64,
        );
        committed.push(continuation);
        assert_replay(&log, &committed, round.cursor).await;
    }
    assert_replay(&log, &committed, 0).await;
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A lost PUT or read-back response cannot invent acknowledgements, overwrite
    /// a different writer's segment, or move replay's cursor past unwritten bytes.
    /// Treating a failed GET as absence or accepting a rival's bytes would break
    /// this property, as would retrying without first reconciling the original PUT.
    #[test]
    fn uncertain_appends_recover_against_the_bucket_history(rounds in prop::collection::vec(round(), 1..12)) {
        tokio::runtime::Builder::new_current_thread().build().expect("runtime")
            .block_on(check_history(&rounds));
    }
}
