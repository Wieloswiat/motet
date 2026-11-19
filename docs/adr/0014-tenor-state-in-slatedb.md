# 0014. Store tenor state in SlateDB and replay from the shard log

Status: Accepted. Builds on [0006](0006-storage.md), which deferred SlateDB.

## Context

[0006](0006-storage.md) stores each shard's history in a fenced log and rebuilds
in-memory state by replaying it from the start. It defers SlateDB for current
state and replay checkpoints.

Hosting World accounts ([0013](0013-tenor-hosts-worlds.md)) adds repo blocks,
record indexes, firehose history, sealed keys, Space repositories and members.
Replaying all of that on every start is too costly. Re-running a repo command
also cannot reproduce its exact commit: signing and revision generation are
not deterministic functions of that command.

## Decision

Store current shard state in SlateDB, but keep the shard log as the only
write-ahead log (WAL).

- Use one SlateDB, pinned to `=0.17.0`, per shard at `state/{shard}/`. Use
  the log's bucket and object-store client. Disable SlateDB's own WAL with
  `wal_disable`, following vlpds.
- Continue logging deterministic rules commands and re-running them on
  replay. For other operations, log the actual key-value mutations and
  firehose events, then reapply those on replay. This includes World
  creation, key custody, repo commits, Space changes and OAuth state.
- Run every operation in order on one writer. Stage its changes in an
  overlay, append the batch as one log segment, then apply it to SlateDB as
  one batch with the applied marker. Use the segment position as SlateDB's
  sequence number. Return outcomes only after these steps. Acknowledgment
  still depends on log durability: SlateDB may lag the log but must never
  contain changes absent from it.
- When state has changed, checkpoint about every ten seconds. Write rules
  snapshots and the applied marker, then flush the memtable to limit the
  work needed on replay.
- Stop the shard if either the log's create-only write or SlateDB's writer
  epoch shows that another writer has taken over. Also stop on any SlateDB
  error after a durable log append; replay repairs the incomplete application.
- Store sealed key material in SlateDB. Seal it with XChaCha20-Poly1305 under
  the tenor's master key, and keep that master key outside the bucket.

## Consequences

SlateDB only promises storage compatibility between adjacent versions.
Keep the exact version pin, upgrade one version at a time and test upgrades
against a stored-format fixture.

SeaweedFS is not on SlateDB's tested store list. Motet's integration tests
must check opening, writing, checkpointing, reopening and writer takeover.

The applied marker makes it possible to compact older log entries, but that
compaction is not built yet.

## Revisit when

SlateDB's pluggable WAL (RFC 0030) becomes stable and can use Motet's log
directly, or multi-node tenors need vlpds's lease and span machinery.
