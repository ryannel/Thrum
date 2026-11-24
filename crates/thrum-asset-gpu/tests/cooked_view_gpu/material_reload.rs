//! The material hook's submitted pixels on one resident rock, for both indexed draw paths.

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedDrawPath, CookedStats, Drawn, MaterialHook, MaterialOutcome,
    MaterialReload, MeshPlacement, await_drawn, draw_endpoint,
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    runtime::{FrameConfig, FrameHost},
    synthetic::SyntheticScene,
};
use thrum_test_support::{Opened, Profile, Restriction, target::SIZE, wait};

use super::support::{Source, World, camera};

type Host = FrameHost<World, Source>;

const LIMIT: Duration = Duration::from_secs(8);
// Keep the fixture's daylight below clipping so the rock's materials differ in the SDR readback.
const EXPOSURE: f32 = 0.0001;

const IDENTITY: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    return surface;
}";

const BANDS: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let band = step(0.5, fract(at.world.x * 3.0));
    out.base_colour = vec4(surface.base_colour.rgb * mix(vec3(1.0), vec3(0.15), band), 1.0);
    out.roughness = 0.95;
    return out;
}";

const GREEN: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    out.base_colour = vec4(0.2, 1.0, 0.3, 1.0);
    out.roughness = 0.3;
    return out;
}";

/// Produce NaN and infinity from a runtime value, in every factor the hook can write.
const NON_FINITE: &str =
    "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let zero = at.uv.x * 0.0;
    out.base_colour = vec4(zero / zero);
    out.roughness = 1.0 / zero;
    out.metalness = zero / zero;
    out.emissive = vec3(1.0 / zero);
    return out;
}";

fn hook(name: &str, source: &str) -> MaterialHook {
    MaterialHook {
        name: name.into(),
        source: source.into(),
    }
}

fn open() -> Option<Opened> {
    thrum_test_support::open(
        "cooked material reload",
        cfg!(target_os = "macos").then_some(wgpu::Backend::Metal),
        Profile::Portable,
        &Restriction::default(),
    )
}

fn select_path(caps: DeviceCapabilities, path: CookedDrawPath) -> DeviceCapabilities {
    if path == CookedDrawPath::Direct {
        caps.without_indirect_execution()
    } else {
        caps
    }
}

struct Fixture {
    host: Host,
    assets: CookedAssets,
}

impl Fixture {
    fn new(queue: &wgpu::Queue, caps: &DeviceCapabilities) -> Self {
        let view = camera([0.4, 1.7, 4.0], [0.0, 0.75, 0.0]);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = view;
        scene.instances[0].deactivate();
        let host = pollster::block_on(FrameHost::new(
            caps,
            queue,
            FrameConfig {
                render_extent: [SIZE; 2],
                surface: None,
            },
            World,
            Source(view),
            scene,
            EXPOSURE,
        ))
        .expect("host");
        let store = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        let rock = MeshPlacement::new("meshes/rock", "textures/white")
            .scale(1.3)
            .on_ground();
        let assets = CookedAssets::new(CookedConfig::new(store, vec![rock])).expect("cooked");
        Self { host, assets }
    }

    fn attach(self, path: CookedDrawPath) -> (Host, Arc<Mutex<CookedStats>>) {
        let Self { mut host, assets } = self;
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach rock");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        (host, stats)
    }
}

fn until_outcome<T>(
    material: &MaterialReload,
    serial: u64,
    mut submit: impl FnMut() -> T,
) -> (T, MaterialOutcome) {
    wait::until("submitted material outcome", LIMIT, || {
        let result = submit();
        material
            .take_outcomes()
            .into_iter()
            .find(|outcome| outcome.serial() == Some(serial))
            .map(|outcome| (result, outcome))
    })
}

fn capture(host: &mut Host) -> Vec<u8> {
    let pixels = host.capture_pixels().expect("submitted pixels");
    assert_eq!(pixels.len(), (SIZE * SIZE * 4) as usize);
    pixels
}

fn promoted_pixels(host: &mut Host, material: &MaterialReload, serial: u64) -> Vec<u8> {
    let ((), outcome) = until_outcome(material, serial, || {
        draw_endpoint(host).expect("submit material reload");
    });
    assert!(
        matches!(outcome, MaterialOutcome::Promoted { .. }),
        "{outcome}"
    );
    capture(host)
}

fn changed_pixels(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(x, y)| x.abs_diff(*y) > 12))
        .count()
}

#[track_caller]
fn assert_visible_change(a: &[u8], b: &[u8]) {
    let changed = changed_pixels(a, b);
    assert!(changed > a.len() / 4 / 100, "only {changed} pixels changed");
}

#[test]
fn requests_before_the_first_frame_draw_the_newest_hook_direct() {
    early_request(CookedDrawPath::Direct);
}

#[test]
fn requests_before_the_first_frame_draw_the_newest_hook_indirect() {
    early_request(CookedDrawPath::Indirect);
}

fn early_request(path: CookedDrawPath) {
    let Some(Opened {
        queue,
        capabilities,
        ..
    }) = open()
    else {
        return;
    };
    let fixture = Fixture::new(&queue, &select_path(capabilities, path));
    let material = fixture.assets.material();
    material.request(hook("early/green.wgsl", GREEN));
    let newest = material.request(hook("early/bands.wgsl", BANDS));
    let (mut host, stats) = fixture.attach(path);

    let (early, outcome) = until_outcome(&material, newest, || capture(&mut host));
    assert!(
        matches!(outcome, MaterialOutcome::Promoted { .. }),
        "{outcome}"
    );
    assert!(stats.lock().expect("stats").drawn.is_some());
    assert_eq!(material.accepted().as_deref(), Some("early/bands.wgsl"));

    let identity = material.request(hook("late/identity.wgsl", IDENTITY));
    let placement = promoted_pixels(&mut host, &material, identity);
    assert_visible_change(&early, &placement);

    let green = material.request(hook("late/green.wgsl", GREEN));
    let superseded_look = promoted_pixels(&mut host, &material, green);
    assert_visible_change(&early, &superseded_look);

    let bands = material.request(hook("late/bands.wgsl", BANDS));
    assert_eq!(early, promoted_pixels(&mut host, &material, bands));
}

#[test]
fn a_hook_that_writes_non_finite_factors_draws_the_placement_material_direct() {
    non_finite(CookedDrawPath::Direct);
}

#[test]
fn a_hook_that_writes_non_finite_factors_draws_the_placement_material_indirect() {
    non_finite(CookedDrawPath::Indirect);
}

fn non_finite(path: CookedDrawPath) {
    let Some(Opened {
        queue,
        capabilities,
        ..
    }) = open()
    else {
        return;
    };
    let fixture = Fixture::new(&queue, &select_path(capabilities, path));
    let material = fixture.assets.material();
    let (mut host, stats) = fixture.attach(path);
    await_drawn(&mut host, &stats, Drawn::Any).expect("resident rock");
    let placement = capture(&mut host);

    let serial = material.request(hook("non-finite.wgsl", NON_FINITE));
    let sanitized = promoted_pixels(&mut host, &material, serial);
    assert_eq!(
        placement, sanitized,
        "non-finite factors changed the placement"
    );
}

#[test]
fn a_replaced_device_draws_the_last_accepted_hook_and_retires_the_old_request_direct() {
    replace_device(CookedDrawPath::Direct);
}

#[test]
fn a_replaced_device_draws_the_last_accepted_hook_and_retires_the_old_request_indirect() {
    replace_device(CookedDrawPath::Indirect);
}

fn replace_device(path: CookedDrawPath) {
    let Some(Opened {
        gpu,
        device,
        queue,
        capabilities,
    }) = open()
    else {
        return;
    };
    let fixture = Fixture::new(&queue, &select_path(capabilities, path));
    let material = fixture.assets.material();
    let (mut host, stats) = fixture.attach(path);
    await_drawn(&mut host, &stats, Drawn::Any).expect("resident rock");
    let placement = capture(&mut host);

    let accepted = material.request(hook("hooks/bands.wgsl", BANDS));
    let before = promoted_pixels(&mut host, &material, accepted);
    assert_eq!(
        stats.lock().expect("stats").drawn_device_generation,
        Some(1)
    );
    assert_visible_change(&placement, &before);

    let outstanding = material.request(hook("hooks/green.wgsl", GREEN));
    let (_device, queue, caps) = gpu
        .fresh_adapter()
        .device_with_hardware(Profile::Portable, &Restriction::default(), 2, &[])
        .expect("open replacement device");
    let caps = select_path(caps, path);
    let inputs = host.device_prepare_inputs().expect("replacement inputs");
    let mut candidate =
        pollster::block_on(inputs.prepare(&caps, &queue, None)).expect("prepared replacement");
    device.destroy();
    host.publish_device(&mut candidate)
        .expect("published replacement");
    assert!(material.restoring());
    assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
    let swaps = stats.lock().expect("stats").swaps.len();

    let ((), retired) = until_outcome(&material, outstanding, || {
        draw_endpoint(&mut host).expect("replacement submission");
    });
    assert_eq!(
        retired,
        MaterialOutcome::Retired {
            serial: outstanding
        }
    );

    wait::until("restaged rock and restored hook", LIMIT, || {
        draw_endpoint(&mut host).expect("replacement submission");
        let restaged = stats.lock().expect("stats").swaps.len() > swaps;
        (restaged && !material.restoring()).then_some(())
    });
    assert_eq!(material.accepted().as_deref(), Some("hooks/bands.wgsl"));
    let restored = capture(&mut host);
    assert_eq!(
        stats.lock().expect("stats").drawn_device_generation,
        Some(2)
    );
    assert_eq!(
        changed_pixels(&before, &restored),
        0,
        "restored hook changed"
    );

    let next = material.request(hook("hooks/green.wgsl", GREEN));
    let reloaded = promoted_pixels(&mut host, &material, next);
    assert_visible_change(&restored, &reloaded);
}
