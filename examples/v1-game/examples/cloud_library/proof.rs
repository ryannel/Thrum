//! Verify the cooked cloud library against the accepted source manifests.
#![cfg(feature = "proving")]

mod authoring;

use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use thrum_asset::{Ktx2, Ktx2Format, ProductKind, Profile};
use thrum_test_support::Scratch;

#[test]
fn cooked_stacks_hold_every_accepted_slab_and_footprint() {
    let root = Scratch::new("cloud-library");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/clouds");
    let store = authoring::cook(
        &source,
        &root.join("volumes"),
        &root.join("store"),
        vec![Profile::Portable],
    )
    .expect("cook both stacks");
    let head = store
        .read_head()
        .expect("read store")
        .expect("published stacks");
    let manifest = store.load_manifest(&head).expect("manifest");
    let library: serde_json::Value =
        serde_json::from_slice(&fs::read(source.join("library.json")).expect("library record"))
            .expect("parse library record");
    let growth: serde_json::Value = serde_json::from_slice(
        &fs::read(source.join("growth/growth-track-manifest.json")).expect("growth record"),
    )
    .expect("parse growth record");
    let shapes = library["shapes"].as_array().expect("shapes");
    let keys = growth["keys"].as_array().expect("keys");
    assert_eq!(shapes.len(), 12);
    for (genus, count) in [("fair-cumulus", 6), ("mediocris", 3), ("congestus", 3)] {
        assert_eq!(
            shapes
                .iter()
                .filter(|shape| shape["genus"] == genus)
                .count(),
            count
        );
    }
    assert_eq!(keys.len(), 11);
    assert!(
        keys.iter()
            .enumerate()
            .all(|(index, key)| key["age_s"] == index * 12)
    );

    let footprints = fs::read(store.root().join("cloud-footprints.bin")).expect("footprints");
    assert_eq!(
        footprints.len(),
        (shapes.len() + keys.len()) * authoring::FOOTPRINT_RECORD_BYTES
    );
    assert_eq!(
        footprints,
        include_bytes!("../../assets/clouds/footprints.bin")
    );
    let (records, remainder) = footprints.as_chunks::<{ authoring::FOOTPRINT_RECORD_BYTES }>();
    assert_eq!(remainder, [0_u8; 0]);
    let mut records = records.iter();
    for (name, slabs) in [("cloud-library", shapes), ("cloud-growth", keys)] {
        let entry = manifest
            .resolve(name, &[Profile::Portable])
            .expect("stack product");
        assert_eq!(entry.kind, ProductKind::Volume);
        let volume =
            Ktx2::open(store.map(&entry.product).expect("stack bytes")).expect("decode KTX2");
        assert_eq!(volume.format(), Ktx2Format::R8Unorm);
        let base = &volume.levels()[0];
        assert_eq!((base.width, base.height), (128, 128));
        assert_eq!(base.depth as usize, 128 * slabs.len());
        let bytes = volume.level_data(base);
        let (voxel_slabs, remainder) = bytes.as_chunks::<{ 128_usize.pow(3) }>();
        assert_eq!(remainder, [0_u8; 0]);
        assert_eq!(voxel_slabs.len(), slabs.len());
        for (voxels, slab) in voxel_slabs.iter().zip(slabs) {
            assert_eq!(
                format!("{:x}", Sha256::digest(voxels)),
                slab["voxel_sha256"].as_str().expect("source identity")
            );
            check_column(records.next().expect("footprint for slab"), voxels);
        }
    }
    assert!(records.next().is_none());
}

fn check_column(footprint: &[u8], voxels: &[u8]) {
    let f32_at = |index: usize| {
        f64::from(f32::from_le_bytes(
            footprint[index * 4..][..4].try_into().expect("float"),
        ))
    };
    let extent = f32_at(0);
    let floor = -f32_at(1) - extent * 0.5;
    let spacing = extent / 128.0;
    // Check the densest column to exercise all three stored measurements, on every slab.
    let (column, sum, bottom, top) = (0..128 * 128)
        .map(|column| {
            let (x, z) = (column % 128, column / 128);
            let density = |y: usize| voxels[(z * 128 + y) * 128 + x];
            let sum = (0..128).map(|y| u32::from(density(y))).sum::<u32>();
            let bottom = (0..128).find(|&y| density(y) > 0);
            let top = (0..128).rfind(|&y| density(y) > 0).map(|y| y + 1);
            (column, sum, bottom, top)
        })
        .max_by_key(|&(_, sum, _, _)| sum)
        .expect("footprint columns");
    let index = 5 + column * 3;
    assert!((f32_at(index) - f64::from(sum) * spacing / 255.0).abs() < 0.001);
    assert!(
        (f32_at(index + 1) - (floor + bottom.expect("occupied") as f64 * spacing)).abs() < 0.001
    );
    assert!((f32_at(index + 2) - (floor + top.expect("occupied") as f64 * spacing)).abs() < 0.001);
}
