//! A source edit becomes visible in a running viewer without the render thread waiting.
//!
//! Short GPU correctness run: cook, start the shipping-shaped viewer, edit the texture and the
//! mesh, and check that each publication is swapped in, drawn, and changes the image while
//! unchanged products are reused rather than re-uploaded. Set `THRUM_G5_CAPTURE_DIR` to keep the
//! before/after frames for visual review.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use asset_loop::{Viewer, ViewerConfig};
use thrum_asset::Profile;
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};

static NEXT: AtomicU32 = AtomicU32::new(0);

fn run_until(viewer: &mut Viewer, generation: u64) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frames = 0;
    while viewer.generation() < generation {
        assert!(
            Instant::now() < deadline,
            "generation {generation} never became resident"
        );
        viewer.frame(0.0);
        frames += 1;
        std::thread::sleep(Duration::from_millis(2));
    }
    frames
}

fn mean_rgb(pixels: &[u8]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for pixel in pixels.as_chunks::<4>().0 {
        for channel in 0..3 {
            sum[channel] += f64::from(pixel[channel]);
        }
    }
    let count = (pixels.len() / 4) as f64;
    sum.map(|value| value / count)
}

fn save(name: &str, extent: [u32; 2], pixels: &[u8]) {
    if let Some(directory) = std::env::var_os("THRUM_G5_CAPTURE_DIR") {
        let path = PathBuf::from(directory).join(name);
        image::save_buffer(
            &path,
            pixels,
            extent[0],
            extent[1],
            image::ExtendedColorType::Rgba8,
        )
        .expect("save capture");
    }
}

#[test]
fn texture_and_mesh_edits_become_visible_without_reuploading_unchanged_products() {
    let root = std::env::temp_dir().join(format!(
        "thrum-g5-visible-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let source = root.join("source");
    std::fs::create_dir_all(source.join("meshes")).expect("create");
    std::fs::create_dir_all(source.join("textures")).expect("create");
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 48, 0.3).expect("gltf");
    demo::write_checker_png(&source.join("textures/checker.png"), 128, [220, 40, 30]).expect("png");
    demo::write_cloud_volume(&source.join("cloud.tvol"), 32, 1).expect("volume");
    let mut cooker = Cooker::open(CookConfig {
        source: source.clone(),
        store: root.join("store"),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open cooker");
    assert!(cooker.cook_all().expect("cook").published.is_some());

    let mut viewer = Viewer::new(ViewerConfig {
        extent: [320, 180],
        ..ViewerConfig::demo(root.join("store"))
    })
    .expect("GPU device");
    run_until(&mut viewer, 1);
    let before = viewer.capture(0.0);
    save("g5-before.png", viewer.extent(), &before);

    // Texture edit: red checker becomes blue.
    let checker = source.join("textures/checker.png");
    demo::write_checker_png(&checker, 128, [30, 40, 220]).expect("edit png");
    let report = cooker
        .cook_changed(&[PathBuf::from("textures/checker.png")])
        .expect("cook edit");
    assert_eq!(report.published.expect("publish").generation, 2);
    run_until(&mut viewer, 2);
    let after_texture = viewer.capture(0.0);
    save("g5-after-texture.png", viewer.extent(), &after_texture);

    // Mesh edit: a taller surface.
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 48, 0.6).expect("edit gltf");
    let report = cooker
        .cook_changed(&[
            PathBuf::from("meshes/terrain.gltf"),
            PathBuf::from("meshes/terrain.bin"),
        ])
        .expect("cook mesh");
    assert_eq!(report.cooked, 1, "only the mesh recooks");
    run_until(&mut viewer, 3);
    let after_mesh = viewer.capture(0.0);
    save("g5-after-mesh.png", viewer.extent(), &after_mesh);
    if std::env::var_os("THRUM_G5_CAPTURE_DIR").is_some() {
        // Orbit sequence for motion review of the final resident set.
        for step in 0..8_u8 {
            let frame = viewer.capture(f32::from(step) * 2.0);
            save(&format!("g5-orbit-{step}.png"), viewer.extent(), &frame);
        }
    }
    viewer.settle();

    let viewer_profiles = (viewer.texture_profiles(), viewer.volume_profiles());
    let records = viewer.records();
    let records = records.lock().expect("records");
    assert!(records.errors.is_empty(), "{:?}", records.errors);
    let swaps: Vec<u64> = records.swaps.iter().map(|swap| swap.generation).collect();
    assert_eq!(swaps, [1, 2, 3]);
    assert!(
        records.swaps.iter().all(|swap| swap.mapped),
        "products are memory-mapped"
    );
    // Every profile is cooked, so the device's first preference is the one drawn.
    let preferred = (viewer_profiles.0[0], viewer_profiles.1[0]);
    assert!(
        records
            .swaps
            .iter()
            .all(|swap| (swap.texture_profile, swap.volume_profile) == preferred),
        "drew {:?}",
        records.swaps[0]
    );
    eprintln!(
        "drawn profiles: texture {:?}, volume {:?}",
        preferred.0, preferred.1
    );
    // Generation 2 re-uploads only the texture; generation 3 only the mesh.
    let [first, texture, mesh] = [0, 1, 2].map(|index| records.swaps[index].uploaded_bytes);
    assert!(
        texture < first && mesh < first,
        "unchanged products are reused: {first} {texture} {mesh}"
    );
    for generation in 1..=3 {
        assert!(
            records.visible(generation).is_some(),
            "generation {generation} completed on the GPU"
        );
    }

    let [r0, _, b0] = mean_rgb(&before);
    let [r1, _, b1] = mean_rgb(&after_texture);
    assert!(
        b1 > b0 + 5.0 && r1 + 5.0 < r0,
        "texture edit visible: before {r0:.1}/{b0:.1}, after {r1:.1}/{b1:.1}"
    );
    let changed = after_texture
        .iter()
        .zip(&after_mesh)
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed > after_mesh.len() / 50,
        "mesh edit visible ({changed} bytes changed)"
    );

    drop(records);
    drop(viewer);
    drop(cooker);
    make_writable(&root);
    std::fs::remove_dir_all(&root).ok();
}

fn make_writable(path: &std::path::Path) {
    if let Ok(meta) = std::fs::metadata(path) {
        let mut permissions = meta.permissions();
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "test cleanup of read-only store files"
        )]
        permissions.set_readonly(false);
        std::fs::set_permissions(path, permissions).ok();
        if meta.is_dir() {
            for entry in std::fs::read_dir(path).into_iter().flatten().flatten() {
                make_writable(&entry.path());
            }
        }
    }
}
