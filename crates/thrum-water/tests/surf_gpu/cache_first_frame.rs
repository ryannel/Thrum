use super::{
    Arc, Bed, Body, BodyKind, Clock, Component, Field, HEIGHT_TOLERANCE, LongWaves, Map, Mutex,
    Profile, QUERY_EXTENT, Sheet, Spectrum, Stage, SurfProfile, SurfaceOutput, TAU, Water,
    query_host, spectrum,
};
use glam::{Mat4, Vec4};
use thrum_render::{runtime::FrameWorld, synthetic::SyntheticView};
use thrum_test_support::readback;
use thrum_water::{SurfTap, SurfWitness};

fn map() -> Map {
    let size = [20, 46];
    let heights = (0..size[1])
        .flat_map(|_| {
            (0..size[0])
                .map(|i| (f64::from(u32::try_from(i).expect("small bed")) * 2.0 - 30.0) * 0.12)
        })
        .collect();
    let mut map = Map::prepare(
        Bed::new([-30.0, -44.0], 2.0, size, heights).expect("plane beach"),
        vec![Body {
            name: "Sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.5,
                value: 0.0,
            },
            fetch: 100_000.0,
            gains: [1.0, 0.0, 0.0, 0.0],
        }],
        thrum_water_model::Profile {
            margin_height: 0.85,
            swash_ratio: 0.7,
        },
    )
    .expect("map");
    map.enable_surf(SurfProfile::default()).expect("transform");
    map
}

fn forcing(gain: f64) -> (Spectrum, Field) {
    let mut spectrum = spectrum();
    spectrum.long = (0..16)
        .map(|i| {
            let i = f64::from(i);
            let (z, x) = (0.17 * i - 1.12).sin_cos();
            let k = TAU / (26.0 + 2.5 * i);
            Component {
                wave_vector: [x * k, z * k],
                amplitude: gain * (0.055 + 0.001 * i),
                phase: 0.37 * i,
            }
        })
        .collect();
    let field = Field {
        layers: std::array::from_fn(|i| {
            LongWaves::new(
                if i == 0 {
                    spectrum.long.clone()
                } else {
                    Vec::new()
                },
                0.2,
            )
            .expect("model waves")
        }),
    };
    (spectrum, field)
}

fn contribution(map: &Arc<Mutex<Map>>, spectrum: Spectrum) -> (Water, SurfTap) {
    let mut profile = Profile::default();
    profile.waves.choppiness = 0.2;
    let mut water = Water::mapped(Arc::clone(map), spectrum, profile).expect("water");
    let tap = water.tap_surf();
    (water, tap)
}

fn view(z: f32, z_direction: f32) -> SyntheticView {
    SyntheticView::perspective(
        [-9.0, 16.0, z],
        [-9.0, 0.0, z + 18.0 * z_direction],
        [0.0, 1.0, 0.0],
        72_f32.to_radians(),
        1.5,
        0.1,
        500.0,
    )
    .expect("camera")
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "project fixture metres with the draw's f32 view"
)]
fn visible_rest(view: Mat4, point: [f64; 2], stage: f64) -> bool {
    let clipped = view * Vec4::new(point[0] as f32, stage as f32, point[1] as f32, 1.0);
    // Well inside the view, with room for the wave displacement and one
    // triangle on either side. The cache is not the visibility oracle.
    clipped.w > 0.0 && clipped.x.abs().max(clipped.y.abs()) < clipped.w * 0.8
}

fn model_displacement(sheet: &impl Sheet, rest: [f64; 2]) -> [f64; 3] {
    let sample = sheet.forward(rest);
    [
        sample.world[0] - rest[0],
        sample.height,
        sample.world[1] - rest[1],
    ]
}

fn compare(actual: &[f32], sheet: &impl Sheet, a: [f64; 2], b: [f64; 2], fraction: f64) -> f64 {
    let first = model_displacement(sheet, a);
    let second = if fraction == 0.0 {
        first
    } else {
        model_displacement(sheet, b)
    };
    actual
        .iter()
        .zip(first.into_iter().zip(second))
        .map(|(actual, (first, second))| {
            let expected = (second - first).mul_add(fraction, first);
            let error = (f64::from(*actual) - expected).abs();
            assert!(
                error < HEIGHT_TOLERANCE,
                "{a:?} to {b:?} ({fraction}): {actual} vs {expected}"
            );
            error
        })
        .fold(0.0, f64::max)
}

#[derive(Clone, Copy)]
struct CheckFrame<'a> {
    name: &'a str,
    map: &'a Map,
    old_map: &'a Map,
    field: &'a Field,
    original_field: &'a Field,
    now: f64,
    before: Option<f64>,
    camera: &'a SyntheticView,
    prior_samples: Option<&'a [[f32; 12]]>,
}

#[expect(
    clippy::float_cmp,
    clippy::print_stderr,
    reason = "exact dyadic mesh spacing and ring edge; record the model error on the target"
)]
fn compare_frame(nodes: &[[f32; 4]], samples: &[[f32; 12]], frame: CheckFrame<'_>) {
    let CheckFrame {
        name,
        map,
        old_map,
        field,
        original_field,
        now,
        before,
        camera,
        prior_samples,
    } = frame;
    let view = Mat4::from_cols_array_2d(&camera.view_proj_columns());
    let [x, _, z] = camera.camera_position().map(f32::floor);
    let anchor = [f64::from(x), f64::from(z)];
    let stage = map.body(0).expect("sea").stage.value;
    let current = map.surf_sheet(field, 0, now).expect("current sheet");
    let previous = before.map(|time| old_map.surf_sheet(field, 0, time).expect("previous sheet"));
    let stale = match name {
        "stage" => Some(
            map.surf_sheet(field, 0, before.expect("stage predecessor"))
                .expect("wrong stage control"),
        ),
        "wind" => Some(
            map.surf_sheet(original_field, 0, now)
                .expect("stale wind control"),
        ),
        _ => None,
    };
    let mut interior = 0;
    let mut edge = 0;
    let mut newly_visible = 0;
    let mut strata = [0; 3];
    let mut maximum = 0.0_f64;
    let mut stale_error = 0.0_f64;
    for (i, (&[x, z, spacing, slot], sample)) in nodes.iter().zip(samples).enumerate() {
        if slot != 0.0 || spacing != 0.25 {
            continue;
        }
        let rest = [anchor[0] + f64::from(x), anchor[1] + f64::from(z)];
        if !visible_rest(view, rest, stage) {
            continue;
        }
        let depth = stage - map.bed().height(rest).expect("camera over the bed");
        let stratum = if depth > 2.0 {
            0
        } else if depth > 0.4 {
            1
        } else {
            2
        };
        if depth < -0.25 {
            continue;
        }
        let boundary = x.abs().max(z.abs()) == 16.0;
        let coarse = [f64::from(x) * 2.0, f64::from(z) * 2.0];
        let fraction = coarse.map(|p| p - p.floor());
        let fraction = fraction[0].max(fraction[1]);
        let (a, b, fraction) = if boundary {
            if fraction == 0.0 {
                continue;
            }
            (
                std::array::from_fn(|axis| anchor[axis] + coarse[axis].floor() * 0.5),
                std::array::from_fn(|axis| anchor[axis] + coarse[axis].ceil() * 0.5),
                fraction,
            )
        } else {
            if strata[stratum] >= 512 {
                continue;
            }
            (rest, rest, 0.0)
        };
        assert!(sample[3] >= 1.0, "{name} first submission omitted {rest:?}");
        maximum = maximum.max(compare(&sample[..3], &current, a, b, fraction));
        if let Some(previous) = &previous {
            maximum = maximum.max(compare(&sample[4..7], previous, a, b, fraction));
        }
        if stale_error <= HEIGHT_TOLERANCE * 2.0
            && !boundary
            && let Some(stale) = &stale
        {
            let height = sample[if name == "stage" { 5 } else { 1 }];
            stale_error =
                stale_error.max((f64::from(height) - model_displacement(stale, rest)[1]).abs());
        }
        if boundary {
            edge += 1;
        } else {
            strata[stratum] += 1;
            interior += 1;
        }
        if prior_samples.is_some_and(|earlier| earlier[i][3] == 0.0) {
            newly_visible += 1;
        }
    }
    eprintln!(
        "{name}: {interior} interior {strata:?}, {edge} blended ring edge, {newly_visible} previously uncached, {maximum:.7} m maximum error"
    );
    assert!(interior >= 1024 && edge >= 32 && strata.iter().all(|count| *count > 0));
    if prior_samples.is_some() {
        assert!(newly_visible >= 32);
    }
    if stale.is_some() {
        assert!(
            stale_error > HEIGHT_TOLERANCE * 2.0,
            "{name} stale control differs by only {stale_error} m"
        );
    }
}

fn read_samples(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    witness: &SurfWitness,
    count: u64,
) -> Vec<[f32; 12]> {
    let bytes = readback::buffer(device, queue, &witness.samples, ..count * 48);
    bytemuck::cast_slice(&bytes).to_vec()
}

#[test]
fn cache_first_frame_after_turn_move_stage_and_wind() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities,
        ..
    }) = thrum_test_support::open(
        "surf first submitted cache",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let (mut host, target) = query_host(
        &device,
        &queue,
        &capabilities,
        Clock::new(false),
        QUERY_EXTENT,
    );
    host.set_timing_requested(false);
    let output = SurfaceOutput {
        view: &target,
        extent: QUERY_EXTENT,
    };
    let map = Arc::new(Mutex::new(map()));
    let (spectrum, field) = forcing(1.0);
    let (water, mut tap) = contribution(&map, spectrum);
    let (_, changed_field) = forcing(1.67);
    host.set_scene_contribution(Box::new(water)).expect("water");
    let step = host.world().tick_duration();
    let mut previous_map = map.lock().expect("test map").clone();
    let mut prior_samples = Vec::new();
    let mut nodes = Vec::new();
    for (tick, (name, camera)) in [
        ("seed", view(-25.0, -1.0)),
        ("turn", view(-25.0, 1.0)),
        ("move", view(25.0, 1.0)),
        ("stage", view(25.0, 1.0)),
        ("wind", view(25.0, 1.0)),
    ]
    .into_iter()
    .enumerate()
    {
        if name == "stage" {
            map.lock()
                .expect("test map")
                .set_stage(0, 0.25)
                .expect("stage");
        }
        if name == "wind" {
            let (water, new_tap) = contribution(&map, forcing(1.67).0);
            host.set_scene_contribution(Box::new(water))
                .expect("changed wind");
            tap = new_tap;
        }
        host.continuous_source_mut().view = camera;
        let elapsed = step * u32::try_from(tick).expect("short test");
        assert!(host.drive(elapsed, Some(&output)).expect("frame").submitted);
        assert!(
            host.contribution_error().is_none(),
            "{:?}",
            host.contribution_error()
        );
        let witness = tap.witness().expect("first submitted field cache");
        if nodes.is_empty() {
            nodes = bytemuck::cast_slice(&readback::buffer(&device, &queue, &witness.nodes, ..))
                .to_vec();
        }
        let samples = read_samples(&device, &queue, &witness, nodes.len() as u64);
        let current_map = map.lock().expect("test map");
        if tick > 0 {
            compare_frame(
                &nodes,
                &samples,
                CheckFrame {
                    name,
                    camera: &camera,
                    map: &current_map,
                    old_map: &previous_map,
                    field: if name == "wind" {
                        &changed_field
                    } else {
                        &field
                    },
                    original_field: &field,
                    now: elapsed.as_secs_f64(),
                    before: (name != "wind").then_some(elapsed.saturating_sub(step).as_secs_f64()),
                    prior_samples: (name == "turn").then_some(&prior_samples[..]),
                },
            );
        }
        previous_map = current_map.clone();
        prior_samples = samples;
        thrum_test_support::wait::until(
            "submitted test frame",
            std::time::Duration::from_secs(4),
            || host.poll_completed().expect("completion"),
        );
    }
}
