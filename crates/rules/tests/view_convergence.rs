//! Consumers fed the same records in different cross-repo orders show the same
//! posts in the same order. (Backend plan, acceptance test 1.)

mod support;

use motet_rules::view::{Op, RepoEvent, Shown, WorldView};
use motet_rules::world::{Admission, Place};
use motet_rules::{Cid, StrongRef};
use motet_testkit::delivery::deliveries;
use proptest::prelude::*;
use support::{Action, SCENES, history, run, scene, world_did};

fn view_of(events: &[RepoEvent]) -> WorldView {
    let mut view = WorldView::new(world_did());
    for event in events {
        view.apply(event);
    }
    view
}

/// A history, plus one way a consumer might receive its records.
fn history_and_delivery() -> impl Strategy<Value = (Vec<Action>, Vec<RepoEvent>)> {
    history().prop_flat_map(|actions| {
        let (_, run) = run(&actions);
        (Just(actions), deliveries(run.streams, 8))
    })
}

/// One published admission amid several versions in the author's repo. The
/// network can deliver the two streams independently and repeat old events.
/// Start each clock at zero, near its maximum, or away from either boundary.
fn admitted_edit() -> impl Strategy<Value = (usize, Vec<RepoEvent>)> {
    let first_rev = || prop_oneof![Just(0), Just(u64::MAX), any::<u64>()];
    (
        1usize..=8,
        any::<prop::sample::Index>(),
        any::<bool>(),
        first_rev(),
        first_rev(),
    )
        .prop_flat_map(|(versions, which, delete, author_rev, world_rev)| {
            let admitted = which.index(versions);
            let author_rev = author_rev.min(u64::MAX - versions as u64);
            let uri = support::post_uri(0, 0);
            let version = |i: usize| StrongRef::new(uri.clone(), Cid::new(format!("cid-{i}")));
            let author = support::author(0);
            let scene = scene(0);
            let mut writes = (0..versions)
                .map(|i| RepoEvent {
                    repo: author.clone(),
                    rev: author_rev + i as u64,
                    op: Op::PutPost {
                        post: version(i),
                        scene: scene.clone(),
                        body: format!("text {i}"),
                    },
                })
                .collect::<Vec<_>>();
            let admission = Admission {
                post: version(admitted),
                author: author.clone(),
                scene,
                place: Place { epoch: 0, seq: 0 },
                authority_rev: 1,
                replaces: None,
            };
            if delete {
                writes.push(RepoEvent {
                    repo: author,
                    rev: author_rev + versions as u64,
                    op: Op::DeletePost { uri },
                });
            }
            let world = RepoEvent {
                repo: world_did(),
                rev: world_rev,
                op: Op::PutAdmission(admission),
            };
            (Just(admitted), deliveries(vec![writes, vec![world]], 4))
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Check each prefix, not only convergence: two readers could agree on the
    /// wrong body. An unadmitted edit must not replace the admitted text, and
    /// an old delivery after deletion must not restore it.
    #[test]
    fn a_reader_sees_only_the_exact_admitted_text((admitted, events) in admitted_edit()) {
        let cid = Cid::new(format!("cid-{admitted}"));
        let mut view = WorldView::new(world_did());
        let (mut admission_seen, mut post_seen, mut version_seen, mut deleted) = (false, false, false, false);
        for e in &events {
            view.apply(e);
            match &e.op {
                Op::PutPost { post, .. } => {
                    post_seen = true;
                    version_seen |= post.cid == cid;
                }
                Op::DeletePost { .. } => deleted = true,
                Op::PutAdmission(_) => admission_seen = true,
                Op::CloseAdmission(_) => unreachable!("the World publishes only the admission"),
            }
            let expected = if !admission_seen || !post_seen {
                None
            } else if deleted {
                Some(Shown::Tombstone)
            } else if version_seen {
                Some(Shown::Version { cid: cid.clone(), body: format!("text {admitted}") })
            } else {
                Some(Shown::Unavailable { cid: cid.clone() })
            };
            let entries = view.scene(&scene(0));
            prop_assert!(entries.len() <= 1);
            prop_assert_eq!(entries.first().map(|e| &e.shown), expected.as_ref(), "after {:?}", e);
        }
    }

    #[test]
    fn any_delivery_order_shows_the_same_scenes((actions, delivered) in history_and_delivery()) {
        let (_, run) = run(&actions);
        let in_order: Vec<RepoEvent> = run.streams.into_iter().flatten().collect();
        let reference = view_of(&in_order);
        let received = view_of(&delivered);
        for s in 0..SCENES {
            prop_assert_eq!(received.scene(&scene(s)), reference.scene(&scene(s)));
        }
    }

    #[test]
    fn scenes_show_exactly_what_the_tenor_admitted(actions in history()) {
        let (world, run) = run(&actions);
        let view = view_of(&run.streams.concat());
        for s in 0..SCENES {
            for entry in view.scene(&scene(s)) {
                // Every non-tombstone entry is the tenor's open admission.
                if let motet_rules::view::Shown::Version { cid, .. } = &entry.shown {
                    let admission = world.admission(&entry.post);
                    prop_assert_eq!(admission.map(|a| &a.post.cid), Some(cid));
                    prop_assert_eq!(admission.map(|a| a.place), Some(entry.place));
                }
            }
        }
        // And every open admission has its entry, at its place.
        for admission in world.admissions() {
            let entries = view.scene(&admission.scene);
            prop_assert!(
                entries.iter().any(|e| e.post == admission.post.uri && e.place == admission.place),
                "admission {admission:?} missing from its scene"
            );
        }
    }

    #[test]
    fn records_count_only_from_the_repo_they_belong_to(actions in history()) {
        let (_, run) = run(&actions);
        let honest: Vec<RepoEvent> = run.streams.concat();
        // A member republishes every admission in their own repo…
        let mut forged = honest.clone();
        for (i, event) in honest.iter().enumerate() {
            if let Op::PutAdmission(a) = &event.op {
                forged.push(RepoEvent {
                    repo: support::author(0),
                    rev: 10_000 + i as u64,
                    op: Op::PutAdmission(a.clone()),
                });
            }
        }
        // And another author "edits" every post in their own repo.
        for (i, event) in honest.iter().enumerate() {
            if let Op::PutPost { post, scene, .. } = &event.op
                && event.repo != support::author(1)
            {
                forged.push(RepoEvent {
                    repo: support::author(1),
                    rev: 20_000 + i as u64,
                    op: Op::PutPost { post: post.clone(), scene: scene.clone(), body: "forged".into() },
                });
            }
        }
        let (a, b) = (view_of(&honest), view_of(&forged));
        for s in 0..SCENES {
            prop_assert_eq!(a.scene(&scene(s)), b.scene(&scene(s)));
        }
    }

    /// An author sees as pending exactly their posts whose current version
    /// the tenor hasn't admitted.
    #[test]
    fn pending_is_whatever_the_tenor_has_not_admitted(actions in history()) {
        let (world, run) = run(&actions);
        let view = view_of(&run.streams.concat());
        for a in 0..support::AUTHORS {
            let author = support::author(a);
            let expected: Vec<_> = run.current.iter()
                .filter(|(uri, _)| uri.as_str().starts_with(&format!("at://{author}/")))
                .filter(|(uri, version)| world.admission(uri).map(|adm| &adm.post) != Some(*version))
                .map(|(_, version)| version.clone())
                .collect();
            prop_assert_eq!(view.pending(&author), expected);
        }
    }
}
