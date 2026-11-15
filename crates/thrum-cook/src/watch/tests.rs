//! A filesystem notification is a hint, not a prerequisite for eventual publication.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use thrum_asset::Profile;

use crate::{CookConfig, Cooker, Durability, Quality, demo};

#[test]
fn silent_event_stream_still_publishes_edits_additions_and_removals() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tmp/review-engine/watcher-silence")
        .join(std::process::id().to_string());
    let source = root.join("source");
    std::fs::create_dir_all(&source).expect("source");
    let original = source.join("original.tvol");
    let added = source.join("new/puff.tvol");
    demo::write_cloud_volume(&original, 8, 1).expect("initial source");
    let mut cooker = Cooker::open(CookConfig {
        source: source.clone(),
        store: root.join("store"),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker");
    let stop = Arc::new(AtomicBool::new(false));
    let (done, finished) = mpsc::channel();
    let deadline = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            // Bound a regression to the old event-only loop, which would otherwise hang.
            finished.recv_timeout(Duration::from_secs(15)).ok();
            stop.store(true, Ordering::Relaxed);
        })
    };
    let (sender, receiver) = mpsc::channel();
    let mut generation = 0;
    let mut products = Vec::new();
    super::run_cycles(&mut cooker, &source, &stop, &receiver, |event| {
        let report = event.report.as_ref().expect("cycle");
        assert!(report.failed.is_empty(), "{report:?}");
        let Some(head) = report.published else {
            return;
        };
        generation = head.generation;
        let store = thrum_asset::AssetStore::new(root.join("store"));
        let manifest = store.load_manifest(&head).expect("manifest");
        match generation {
            1 => {
                products.push(
                    manifest
                        .resolve("original", &[Profile::Portable])
                        .expect("entry")
                        .product,
                );
                demo::write_cloud_volume(&original, 8, 2).expect("edit");
            }
            2 => {
                assert_ne!(
                    manifest
                        .resolve("original", &[Profile::Portable])
                        .expect("entry")
                        .product,
                    products[0]
                );
                std::fs::create_dir_all(added.parent().expect("parent")).expect("new directory");
                demo::write_cloud_volume(&added, 8, 3).expect("add");
            }
            3 => {
                assert!(manifest.resolve("new/puff", &[Profile::Portable]).is_some());
                std::fs::remove_file(&original).expect("remove");
            }
            4 => {
                assert!(manifest.resolve("original", &[Profile::Portable]).is_none());
                assert!(manifest.resolve("new/puff", &[Profile::Portable]).is_some());
                stop.store(true, Ordering::Relaxed);
            }
            _ => panic!("unexpected generation {generation}"),
        }
    })
    .expect("cycles");
    done.send(()).ok();
    deadline.join().expect("deadline");
    drop(sender);
    assert_eq!(generation, 4, "all changes publish without a single event");
    // The store deliberately makes CAS files read-only; its owner directory can still unlink
    // them on this Mac, the target of the repository gate.
    std::fs::remove_dir_all(root).expect("cleanup");
}
