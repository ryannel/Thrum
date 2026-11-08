//! G6 material hook in the game: a game-authored WGSL hook reloads explicitly into the running
//! scene's cooked meshes, an invalid hook leaves the last pixels drawing with a diagnostic in the
//! hook's own file and line, a burst of requests compiles only the outstanding and the newest one,
//! and a device replaced while a reload is outstanding draws the last accepted hook once it
//! validates on the new device. `THRUM_G6_MATERIAL_CAPTURE_DIR` keeps the captures as PPM files.
#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thrum_asset::Profile;
use thrum_asset_gpu::{CookedAssets, CookedStats, MaterialHook, MaterialOutcome, MaterialReload};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::present::SurfaceProfile;
use v1_game::{FrameConfig, FrameHost, SurfaceOutput};

const EXTENT: [u32; 2] = [640, 360];

const IDENTITY: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    return surface;
}
";

/// Dark bands across the world X axis, and a rough surface.
const BANDS: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let band = step(0.5, fract(at.world.x * 3.0));
    out.base_colour = vec4(surface.base_colour.rgb * mix(vec3(1.0), vec3(0.15), band), 1.0);
    out.roughness = 0.95;
    return out;
}
";

/// Green and smooth: a second valid look.
const GREEN: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    out.base_colour = vec4(0.2, 1.0, 0.3, 1.0);
    out.roughness = 0.3;
    return out;
}
";

/// A misspelt identifier on line 3, column 21.
const MISSPELT: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    out.roughness = roughnes * 0.5;
    return out;
}
";

/// Every factor infinite or NaN, computed at run time so no compiler folds it away.
const NON_FINITE: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let zero = at.uv.x * 0.0;
    out.base_colour = vec4(zero / zero);
    out.roughness = 1.0 / zero;
    out.metalness = zero / zero;
    out.emissive = vec3(1.0 / zero);
    return out;
}
";

fn hook(name: &str, source: &str) -> MaterialHook {
    MaterialHook {
        name: name.to_owned(),
        source: source.to_owned(),
    }
}

fn source_tree(root: &Path) -> CookConfig {
    let source = root.join("source");
    for directory in ["meshes", "textures"] {
        std::fs::create_dir_all(source.join(directory)).expect("source tree");
    }
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 128, 0.25).expect("mesh");
    demo::write_checker_png(&source.join("textures/checker.png"), 512, [200, 200, 200])
        .expect("texture");
    demo::write_cloud_volume(&source.join("cloud.tvol"), 32, 1).expect("volume");
    CookConfig {
        source,
        store: root.join("store"),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    }
}

fn cooked(name: &str) -> (PathBuf, CookConfig) {
    let root = std::env::temp_dir().join(format!("thrum-g6-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root);
    let report = Cooker::open(cook.clone())
        .expect("cooker")
        .cook_all()
        .expect("cook");
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    (root, cook)
}

async fn gpu(
    instance: &wgpu::Instance,
    generation: u64,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("real adapter required for the in-game G6 proof");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_content_features(&adapter)
                | DeviceCapabilities::requested_timing_features(&adapter),
            ..Default::default()
        })
        .await
        .expect("device");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
    (device, queue, capabilities)
}

fn settle(host: &mut FrameHost, device: &wgpu::Device) {
    while host.pending_frames() > 0 {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU wait");
        host.poll_completed().expect("GPU progress");
    }
}

/// Drive one submitted live frame and wait for it. Steps are 16 ms against the 60 Hz tick, so about
/// one step in 25 completes no tick, and a live frame without a completed tick submits nothing; such
/// a step leaves the last frame's stats in place, so keep stepping until a frame is submitted.
fn live_frame(host: &mut FrameHost, device: &wgpu::Device, clock: &mut Duration) {
    for _ in 0..4 {
        *clock += Duration::from_millis(16);
        let submitted = host.drive(*clock, None).expect("frame").submitted;
        settle(host, device);
        if submitted {
            return;
        }
    }
    panic!("no live frame submitted in four 16 ms steps");
}

/// Drive live frames until the cooked set `generation` is drawn and one more frame follows it.
fn drive_until_drawn(
    host: &mut FrameHost,
    device: &wgpu::Device,
    stats: &Mutex<CookedStats>,
    clock: &mut Duration,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let drawn = stats.lock().expect("stats").drawn.is_some();
        let errors = stats.lock().expect("stats").errors.clone();
        assert!(errors.is_empty(), "loader errors: {errors:?}");
        assert!(Instant::now() < deadline, "cooked set never drawn");
        live_frame(host, device, clock);
        if drawn {
            return;
        }
    }
}

/// Render frames (captures or live frames) until `serial` has an outcome; returns it and every
/// outcome seen meanwhile.
fn until_outcome(
    material: &MaterialReload,
    serial: u64,
    mut frame: impl FnMut(),
) -> (MaterialOutcome, Vec<MaterialOutcome>) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut seen = Vec::new();
    loop {
        frame();
        for outcome in material.take_outcomes() {
            record(&outcome);
            seen.push(outcome);
        }
        if let Some(outcome) = seen.iter().find(|outcome| outcome.serial() == Some(serial)) {
            return (outcome.clone(), seen);
        }
        assert!(
            Instant::now() < deadline,
            "reload {serial} had no outcome: {seen:?}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn changed_pixels(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(x, y)| x.abs_diff(*y) > 12))
        .count()
}

fn save(name: &str, pixels: &[u8]) {
    if let Some(directory) = std::env::var_os("THRUM_G6_MATERIAL_CAPTURE_DIR") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("capture directory");
        let mut ppm = format!("P6\n{} {}\n255\n", EXTENT[0], EXTENT[1]).into_bytes();
        for pixel in pixels.as_chunks::<4>().0 {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(directory.join(format!("{name}.ppm")), ppm).expect("capture");
    }
}

/// Keep every outcome's text beside the captures, for review.
fn record(outcome: &MaterialOutcome) {
    if let Some(directory) = std::env::var_os("THRUM_G6_MATERIAL_CAPTURE_DIR") {
        use std::io::Write as _;
        std::fs::create_dir_all(&directory).expect("capture directory");
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(PathBuf::from(directory).join("outcomes.txt"))
            .expect("outcome log");
        writeln!(log, "{outcome}").expect("outcome");
    }
}

fn capture(host: &mut FrameHost, name: &str) -> Vec<u8> {
    let pixels = host.capture_pixels().expect("capture");
    save(name, &pixels);
    pixels
}

#[test]
fn material_hooks_reload_visibly_and_invalid_hooks_keep_the_last_pixels() {
    let (root, cook) = cooked("material");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (device, queue, capabilities) = gpu(&instance, FrameHost::DEVICE_GENERATION).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                surface: None,
            },
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats: Arc<Mutex<CookedStats>> = assets.stats();
        let material = assets.material();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        drive_until_drawn(&mut host, &device, &stats, &mut clock);
        assert_eq!(stats.lock().expect("stats").last_frame_history, Some(true));

        // Live frames: the promoting frame restarts motion history once, the next continues.
        let serial = material.request(hook("live/identity.wgsl", IDENTITY));
        let mut histories = Vec::new();
        let (outcome, _) = until_outcome(&material, serial, || {
            clock += Duration::from_millis(16);
            host.drive(clock, None).expect("frame");
            settle(&mut host, &device);
            histories.push(stats.lock().expect("stats").last_frame_history);
        });
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        assert_eq!(histories.last(), Some(&Some(false)), "{histories:?}");
        // A live step that completes no tick submits nothing and keeps the promoting frame's stats.
        assert!(!host.drive(clock, None).expect("no-tick step").submitted);
        settle(&mut host, &device);
        assert_eq!(stats.lock().expect("stats").last_frame_history, Some(false));
        live_frame(&mut host, &device, &mut clock);
        assert_eq!(stats.lock().expect("stats").last_frame_history, Some(true));

        // Paused from here, so captures differ only by material.
        host.pause().expect("pause");
        let identity = capture(&mut host, "identity");
        assert_eq!(
            identity,
            host.capture_pixels().expect("capture"),
            "paused captures repeat"
        );

        // A visible valid reload.
        let serial = material.request(hook("hooks/bands.wgsl", BANDS));
        let (outcome, _) = until_outcome(&material, serial, || drop(capture(&mut host, "_")));
        let MaterialOutcome::Promoted { name, timing, .. } = &outcome else {
            panic!("bands refused: {outcome}");
        };
        assert_eq!(name, "hooks/bands.wgsl");
        assert!(timing.prepare <= timing.eligible && timing.eligible <= timing.drawn);
        assert_eq!(material.accepted().as_deref(), Some("hooks/bands.wgsl"));
        let bands = capture(&mut host, "bands");
        let changed = changed_pixels(&identity, &bands);
        assert!(
            changed > 640 * 360 / 100,
            "the hook is visible: {changed} pixels changed"
        );

        // An invalid reload: the bands keep drawing and the diagnostic names the hook's line.
        let serial = material.request(hook("hooks/misspelt.wgsl", MISSPELT));
        let (outcome, _) = until_outcome(&material, serial, || drop(capture(&mut host, "_")));
        let MaterialOutcome::Rejected { diagnostics, .. } = &outcome else {
            panic!("a misspelt hook was accepted: {outcome}");
        };
        let first = diagnostics.first().expect("a diagnostic");
        assert_eq!(
            (first.file.as_str(), first.line, first.column),
            ("hooks/misspelt.wgsl", 3, 21),
            "{outcome}"
        );
        assert!(first.message.contains("roughnes"), "{outcome}");
        assert_eq!(material.accepted().as_deref(), Some("hooks/bands.wgsl"));
        let after_invalid = capture(&mut host, "after-invalid");
        assert_eq!(
            changed_pixels(&bands, &after_invalid),
            0,
            "the last pixels keep drawing"
        );

        // A missing hook function is reported at the glue, and a stale request cannot publish.
        let missing = material.request(hook("hooks/empty.wgsl", "// nothing here\n"));
        let superseded = material.request(hook("hooks/identity.wgsl", IDENTITY));
        let newest = material.request(hook("hooks/green.wgsl", GREEN));
        let (outcome, seen) = until_outcome(&material, newest, || drop(capture(&mut host, "_")));
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        for serial in [missing, superseded] {
            assert!(
                seen.iter().any(|outcome| outcome.serial() == Some(serial)
                    && matches!(outcome, MaterialOutcome::Superseded { .. })),
                "{serial} was not superseded: {seen:?}"
            );
        }
        let green = capture(&mut host, "green");
        assert!(changed_pixels(&bands, &green) > 640 * 360 / 100);

        let missing = material.request(hook("hooks/empty.wgsl", "// nothing here\n"));
        let (outcome, _) = until_outcome(&material, missing, || drop(capture(&mut host, "_")));
        let MaterialOutcome::Rejected { diagnostics, .. } = &outcome else {
            panic!("a hook without cooked_surface was accepted: {outcome}");
        };
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.line == 0 && diagnostic.file == "hooks/empty.wgsl"),
            "{outcome}"
        );
        assert_eq!(
            changed_pixels(&green, &capture(&mut host, "after-missing")),
            0
        );
    });
    std::fs::remove_dir_all(&root).ok();
}

/// The no-hook default and the identity hook draw the same pixels.
#[test]
fn the_identity_hook_draws_the_built_in_material() {
    let (root, cook) = cooked("identity");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (device, queue, capabilities) = gpu(&instance, FrameHost::DEVICE_GENERATION).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                surface: None,
            },
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats = assets.stats();
        let material = assets.material();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        drive_until_drawn(&mut host, &device, &stats, &mut clock);
        host.pause().expect("pause");
        let default = capture(&mut host, "default");
        let serial = material.request(hook("identity.wgsl", IDENTITY));
        let (outcome, _) = until_outcome(&material, serial, || drop(capture(&mut host, "_")));
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        let identity = capture(&mut host, "identity-hook");
        let differing = default
            .iter()
            .zip(&identity)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differing, 0, "identity hook changed {differing} bytes");
    });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_hook_that_writes_non_finite_factors_draws_the_placement_material() {
    let (root, cook) = cooked("non-finite");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (device, queue, capabilities) = gpu(&instance, FrameHost::DEVICE_GENERATION).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                surface: None,
            },
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats = assets.stats();
        let material = assets.material();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        drive_until_drawn(&mut host, &device, &stats, &mut clock);
        host.pause().expect("pause");
        let default = capture(&mut host, "default");
        let serial = material.request(hook("non-finite.wgsl", NON_FINITE));
        let (outcome, _) = until_outcome(&material, serial, || drop(capture(&mut host, "_")));
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        let sanitized = capture(&mut host, "non-finite-hook");
        let differing = default
            .iter()
            .zip(&sanitized)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differing, 0, "non-finite factors changed {differing} bytes");
    });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn requests_before_the_first_frame_wait_for_the_scene_and_the_newest_draws() {
    let (root, cook) = cooked("material-early");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (device, queue, capabilities) = gpu(&instance, FrameHost::DEVICE_GENERATION).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                surface: None,
            },
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats = assets.stats();
        let material = assets.material();
        let first = material.request(hook("early/green.wgsl", GREEN));
        let second = material.request(hook("early/bands.wgsl", BANDS));
        assert_eq!(
            material.take_outcomes(),
            vec![MaterialOutcome::Superseded { serial: first }]
        );
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        let (outcome, _) = until_outcome(&material, second, || {
            clock += Duration::from_millis(16);
            host.drive(clock, None).expect("frame");
            settle(&mut host, &device);
        });
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        assert_eq!(material.accepted().as_deref(), Some("early/bands.wgsl"));
        assert!(stats.lock().expect("stats").drawn.is_some());
    });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_burst_during_a_compile_compiles_only_the_outstanding_and_the_newest_request() {
    let (root, cook) = cooked("material-burst");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (device, queue, capabilities) = gpu(&instance, FrameHost::DEVICE_GENERATION).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                surface: None,
            },
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats = assets.stats();
        let material = assets.material();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        drive_until_drawn(&mut host, &device, &stats, &mut clock);
        host.pause().expect("pause");
        // The first request starts the one compile; each later one replaces the waiting request,
        // which is superseded at once, before it compiles.
        let outstanding = material.request(hook("burst/bands.wgsl", BANDS));
        let waiting: Vec<u64> = [
            "burst/identity.wgsl",
            "burst/missing.wgsl",
            "burst/again.wgsl",
        ]
        .into_iter()
        .map(|name| material.request(hook(name, IDENTITY)))
        .collect();
        let newest = material.request(hook("burst/green.wgsl", GREEN));
        assert_eq!(
            material.take_outcomes(),
            waiting
                .iter()
                .map(|&serial| MaterialOutcome::Superseded { serial })
                .collect::<Vec<_>>(),
            "waiting requests are superseded without compiling"
        );
        let (outcome, seen) = until_outcome(&material, newest, || drop(capture(&mut host, "_")));
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        assert!(
            seen.contains(&MaterialOutcome::Superseded {
                serial: outstanding
            }),
            "the outstanding compile cannot publish: {seen:?}"
        );
        assert_eq!(seen.len(), 2, "one outcome each: {seen:?}");
        assert_eq!(material.accepted().as_deref(), Some("burst/green.wgsl"));
        settle(&mut host, &device);
    });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_replaced_device_draws_the_last_accepted_hook_and_retires_the_outstanding_reload() {
    let (root, cook) = cooked("material-device");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (old_device, old_queue, old_capabilities) = gpu(&instance, 1).await;
        let config = FrameConfig {
            render_extent: EXTENT,
            surface: None,
        };
        let mut host =
            FrameHost::new_with_capabilities(&old_device, &old_queue, &old_capabilities, config)
                .await
                .expect("prepared game scene");
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats = assets.stats();
        let material = assets.material();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        drive_until_drawn(&mut host, &old_device, &stats, &mut clock);
        host.pause().expect("pause");
        let accepted = material.request(hook("hooks/bands.wgsl", BANDS));
        let (outcome, _) = until_outcome(&material, accepted, || {
            drop(capture(&mut host, "_"));
        });
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        let before = capture(&mut host, "device-before");

        // Request a reload, then replace the device before any frame could draw it.
        let outstanding = material.request(hook("hooks/green.wgsl", GREEN));
        let (device, queue, capabilities) = gpu(&instance, 2).await;
        let mut candidate = host
            .device_prepare_inputs()
            .expect("inputs")
            .prepare(&capabilities, &queue, None)
            .await
            .expect("replacement");
        old_device.destroy();
        host.publish_device(&mut candidate)
            .expect("publish replacement");
        assert!(
            material.restoring(),
            "the replacement draws the built-in material until the hook validates"
        );
        let swaps = stats.lock().expect("stats").swaps.len();
        let (outcome, _) = until_outcome(&material, outstanding, || {
            drop(capture(&mut host, "_"));
        });
        assert!(
            matches!(outcome, MaterialOutcome::Retired { .. }),
            "{outcome}"
        );
        // The same set is staged again for the new device, and drawn with the accepted hook.
        let deadline = Instant::now() + Duration::from_secs(30);
        while stats.lock().expect("stats").swaps.len() == swaps {
            assert!(Instant::now() < deadline, "the set never restaged");
            drop(capture(&mut host, "_"));
        }
        assert_eq!(material.accepted().as_deref(), Some("hooks/bands.wgsl"));
        // The accepted hook compiles again for the new device through the one compile owner,
        // after the outstanding old-device compile, and draws once valid and submitted.
        let deadline = Instant::now() + Duration::from_secs(30);
        while material.restoring() {
            assert!(
                Instant::now() < deadline,
                "the accepted hook was never restored"
            );
            drop(capture(&mut host, "_"));
            std::thread::sleep(Duration::from_millis(2));
        }
        let after = capture(&mut host, "device-after");
        assert_eq!(
            changed_pixels(&before, &after),
            0,
            "the new device draws the last accepted hook"
        );

        // A reload on the new device then promotes as usual.
        let again = material.request(hook("hooks/green.wgsl", GREEN));
        let (outcome, _) = until_outcome(&material, again, || drop(capture(&mut host, "_")));
        assert!(
            matches!(outcome, MaterialOutcome::Promoted { .. }),
            "{outcome}"
        );
        let green = capture(&mut host, "device-green");
        assert!(changed_pixels(&after, &green) > 640 * 360 / 100);
        settle(&mut host, &device);
    });
    std::fs::remove_dir_all(&root).ok();
}

/// A live step longer than the 60 Hz tick, so every step completes a tick and submits (or, when
/// encoded and skipped, drops) a view.
const STEP: Duration = Duration::from_millis(17);

/// A material validated before any cooked content is resident is ready, not drawn: live frames
/// and a capture submitted without a cooked mesh leave it without an outcome. The first
/// submission whose frame recorded a resident mesh with it promotes it, after the first set's
/// swap.
#[test]
fn a_material_ready_before_content_is_drawn_only_with_the_first_mesh() {
    let root = std::env::temp_dir().join(format!("thrum-g6-material-ready-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root);
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (device, queue, capabilities) = gpu(&instance, FrameHost::DEVICE_GENERATION).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                surface: None,
            },
        )
        .await
        .expect("prepared game scene");
        // Nothing is cooked yet: the store has no HEAD, so no set can become resident.
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats = assets.stats();
        let material = assets.material();
        let requested = Instant::now();
        let serial = material.request(hook("ready/green.wgsl", GREEN));
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        let deadline = Instant::now() + Duration::from_secs(30);
        while material.ready() != Some(serial) {
            assert!(Instant::now() < deadline, "the material never became ready");
            clock += STEP;
            host.drive(clock, None).expect("frame");
            settle(&mut host, &device);
            std::thread::sleep(Duration::from_millis(2));
        }
        // Submissions without a cooked mesh: more live frames and a capture.
        let submitted = stats.lock().expect("stats").submitted_frames;
        for _ in 0..3 {
            clock += STEP;
            host.drive(clock, None).expect("frame");
            settle(&mut host, &device);
        }
        capture(&mut host, "ready-before-content");
        assert!(stats.lock().expect("stats").submitted_frames >= submitted + 4);
        assert_eq!(material.take_outcomes(), Vec::<MaterialOutcome>::new());
        assert_eq!(material.ready(), Some(serial));
        assert_eq!(material.accepted(), None);
        assert!(stats.lock().expect("stats").drawn.is_none());
        // Publish content: the first frame that draws it draws the material.
        let report = Cooker::open(cook.clone())
            .expect("cooker")
            .cook_all()
            .expect("cook");
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        let (outcome, _) = until_outcome(&material, serial, || {
            clock += STEP;
            host.drive(clock, None).expect("frame");
            settle(&mut host, &device);
        });
        let MaterialOutcome::Promoted { timing, .. } = outcome else {
            panic!("{outcome}");
        };
        let swapped = stats
            .lock()
            .expect("stats")
            .swaps
            .first()
            .expect("a swap")
            .swapped;
        assert!(
            requested + timing.drawn >= swapped,
            "drawn {:?} before the first set swapped",
            timing.drawn
        );
        assert_eq!(material.ready(), None);
        assert_eq!(material.accepted().as_deref(), Some("ready/green.wgsl"));
    });
    std::fs::remove_dir_all(&root).ok();
}

/// With a mesh resident, an encoded frame that is skipped (as the web shell does when drawable
/// acquisition fails) draws the held material in a recording that is never submitted: it
/// promotes nothing and leaves the material ready. The next submitted frame promotes it once.
#[test]
fn a_skipped_frame_never_promotes_a_ready_material() {
    let (root, cook) = cooked("material-skip");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (device, queue, capabilities) = gpu(&instance, FrameHost::DEVICE_GENERATION).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: EXTENT,
                // A presenter, so the scene can be encoded before a drawable and skipped.
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent: EXTENT,
                }),
            },
        )
        .await
        .expect("prepared game scene");
        let drawable = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("skip test drawable"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = drawable.create_view(&wgpu::TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        let present = |host: &mut FrameHost, clock: &mut Duration| {
            *clock += STEP;
            assert!(host.drive(*clock, Some(&output)).expect("frame").submitted);
            settle(host, &device);
        };
        let assets = CookedAssets::new(v1_game::cooked::cooked_config(cook.store.clone()))
            .expect("rigid placements");
        let stats = assets.stats();
        let material = assets.material();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        let deadline = Instant::now() + Duration::from_secs(30);
        while stats.lock().expect("stats").drawn.is_none() {
            assert!(Instant::now() < deadline, "cooked set never drawn");
            present(&mut host, &mut clock);
        }
        present(&mut host, &mut clock);
        let serial = material.request(hook("skip/green.wgsl", GREEN));
        let submitted = stats.lock().expect("stats").submitted_frames;
        let skip = |host: &mut FrameHost, clock: &mut Duration| {
            *clock += STEP;
            let frame = host.advance(*clock).expect("advance");
            assert!(frame.encode_scene().expect("encode").skip().dropped_view);
        };
        // Skipped frames until the validated material is held, then one more: each records the
        // resident mesh with it.
        let deadline = Instant::now() + Duration::from_secs(30);
        while material.ready() != Some(serial) {
            assert!(Instant::now() < deadline, "the material never became ready");
            skip(&mut host, &mut clock);
            std::thread::sleep(Duration::from_millis(2));
        }
        skip(&mut host, &mut clock);
        assert_eq!(stats.lock().expect("stats").submitted_frames, submitted);
        assert_eq!(material.take_outcomes(), Vec::<MaterialOutcome>::new());
        assert_eq!(material.ready(), Some(serial));
        assert_eq!(material.accepted(), None);
        // The next submitted frame draws it with the mesh and promotes it, once.
        present(&mut host, &mut clock);
        let outcomes = material.take_outcomes();
        assert!(
            matches!(&outcomes[..], [MaterialOutcome::Promoted { serial: promoted, .. }] if *promoted == serial),
            "{outcomes:?}"
        );
        for _ in 0..2 {
            present(&mut host, &mut clock);
        }
        assert_eq!(material.take_outcomes(), Vec::<MaterialOutcome>::new());
        assert_eq!(material.ready(), None);
        assert_eq!(material.accepted().as_deref(), Some("skip/green.wgsl"));
    });
    std::fs::remove_dir_all(&root).ok();
}
