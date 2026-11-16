//! Paired CPU preparation of the shipping loader's actual mesh upload sources.
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Instant;
use thrum_asset::{AssetStore, MeshProduct, ProductKind};
use thrum_asset_gpu::mesh_upload::MeshUpload;

pub(super) fn measure(report: &mut String) {
    let Some(path) = std::env::var_os("THRUM_MESH_BENCH_STORE") else {
        return;
    };
    let store = AssetStore::new(path);
    let head = store.read_head().expect("HEAD").expect("cooked store");
    let manifest = store.load_manifest(&head).expect("manifest");
    writeln!(report, "\n## Mapped mesh upload preparation\n\n100 warm repetitions; time includes mapping, structural open and upload-source preparation; allocated bytes are owned index/range capacities, excluding mappings, driver and staging.\n\n| mesh | indices | meshlets | median us | allocated bytes |\n|---|---|---|---|---|").ok();
    for entry in manifest.entries().filter(|e| e.kind == ProductKind::Mesh) {
        let mut times = Vec::new();
        let mut row = (0, 0, 0);
        for _ in 0..100 {
            let start = Instant::now();
            let mesh =
                Arc::new(MeshProduct::open(store.map(&entry.product).expect("map")).expect("mesh"));
            let upload = MeshUpload::new(mesh).expect("meshlet coverage");
            row = (
                upload.indices().len(),
                upload.ranges().len(),
                upload.allocated_bytes(),
            );
            std::hint::black_box(&upload);
            times.push(start.elapsed().as_secs_f64() * 1e6);
        }
        times.sort_by(f64::total_cmp);
        writeln!(
            report,
            "| {} | {} | {} | {:.3} | {} |",
            entry.name, row.0, row.1, times[50], row.2
        )
        .ok();
    }
}
