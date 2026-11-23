//! A leaning, internally uneven volume exercises the same density in both GPU marches.

#[cfg(target_os = "macos")]
mod placed_fixture;
#[cfg(target_os = "macos")]
mod scene_fixture;

thrum_test_support::macos_gpu_test! {
    fn a_twenty_degree_sun_sees_the_leaning_volume() {
        run(20.0);
    }
}
thrum_test_support::macos_gpu_test! {
    fn a_sixty_degree_sun_sees_the_leaning_volume() {
        run(60.0);
    }
}

#[cfg(target_os = "macos")]
fn run(elevation: f32) {
    use placed_fixture::Fixture;
    let Some(gpu) = thrum_test_support::open(
        "placed Sun and trace",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let fixture = Fixture::new();
    let field = thrum_cloud_model::Field::new(
        fixture.config.seed,
        fixture.config.model,
        fixture.config.types.clone(),
    )
    .expect("model");
    let radians = elevation.to_radians();
    let sun = [radians.cos(), radians.sin(), 0.0];
    let arms = [(1.0, [25.0, 40.0]), (0.5, [0.0, 40.0]), (0.0, [0.0, 10.0])];
    let mut controls = Vec::new();
    pollster::block_on(async {
        for (weight, range) in arms {
            let (_host, input) = fixture.submit(&gpu, range, true, true, sun).await;
            controls.push(compare_sun(&gpu, &input, &fixture, &field, sun, weight));
            compare_trace(&gpu, &input, &field, &fixture, weight);
        }
    });
    assert!(
        controls[0]
            .iter()
            .zip(&controls[2])
            .any(|(a, b)| (a - b).abs() > 0.03),
        "a footprint substitution must fail on the oblique probes"
    );
}

#[cfg(target_os = "macos")]
fn compare_sun(
    gpu: &thrum_test_support::Opened,
    input: &thrum_clouds::WeatherInput,
    fixture: &placed_fixture::Fixture,
    field: &thrum_cloud_model::Field,
    sun: [f32; 3],
    weight: f64,
) -> Vec<f64> {
    use thrum_cloud_model::{Density, Sample};
    let (_, buffer) = input.ground_product().expect("mapping");
    let bytes = thrum_test_support::readback::buffer(&gpu.device, &gpu.queue, &buffer, ..);
    let map: thrum_render::atmosphere::GpuSunMapping = bytemuck::pod_read_unaligned(&bytes);
    let raw = placed_fixture::ground(gpu, input);
    let side = placed_fixture::GROUND;
    let direction = sun.map(f64::from);
    let spacing = f64::from(map.near[2]) / f64::from(side / 2);
    let mut values = Vec::new();
    for z in 4..12 {
        for x in 1..15 {
            let texture_before = usize::try_from((z * side + x) * 8).expect("texel");
            let actual = fp16(&raw[texture_before..][..2]);
            let xx = f64::from(map.near[0]) + (f64::from(x) + 0.5) * spacing;
            let zz = f64::from(map.near[1]) + (f64::from(z) + 0.5) * spacing;
            let mut expected = 0.0;
            // A ground texel averages four cached rays at half a path texel.
            for dx in [-5.0, 5.0] {
                for dz in [-5.0, 5.0] {
                    let ray = Sample {
                        tick: fixture.state.tick(),
                        origin: [xx + dx, f64::from(map.plane_drift[0]), zz + dz],
                        direction,
                        length: 160.0 / direction[1],
                        steps: 768,
                    };
                    expected += field.optical_depth(
                        &ray,
                        &fixture.state,
                        Density::Uneroded,
                        |_| Some(&fixture.shape),
                        weight,
                    ) * 0.25;
                }
            }
            // The cache filters discretely stepped, trilinear R8 samples. The model uses exact
            // vertical slabs. Their absolute tolerance is small beside a second column.
            assert!(
                (actual - expected).abs() < 0.10 + 0.15 * expected,
                "Sun {sun:?}, weight {weight}: {x}, {z}: {actual} versus {expected}"
            );
            values.push(actual);
        }
    }
    assert!(
        values.iter().any(|&value| value > 0.1),
        "the selected body casts shade"
    );
    values
}

#[cfg(target_os = "macos")]
fn compare_trace(
    gpu: &thrum_test_support::Opened,
    input: &thrum_clouds::WeatherInput,
    field: &thrum_cloud_model::Field,
    fixture: &placed_fixture::Fixture,
    weight: f64,
) {
    use thrum_cloud_model::{Density, Sample};
    let sky = input.sky_products().expect("submitted trace");
    let texture = &sky.fresh_transport;
    let bytes = thrum_test_support::readback::texture(
        &gpu.device,
        &gpu.queue,
        texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    );
    let side = texture.width() as usize;
    let index = ((side / 2) * side + side / 2) * 8;
    assert_eq!(side % 2, 1, "one trace ray is exactly vertical");
    let actual = -fp16(&bytes[index + 6..index + 8]).ln();
    let expected = field.query([0.0; 3], &fixture.state).optical_depth;
    let allowance = 0.09;
    assert!(actual.is_finite() && actual > expected * 0.60);
    assert!(
        actual <= expected + allowance,
        "drawn {actual} exceeds headless {expected}"
    );
    assert!(
        actual * 2.0 > expected + allowance,
        "double density must fail"
    );
    // Reconstruct off-axis pixel rays in the camera's -X/right and -Z/up
    // basis. Require an occupied core and clear air at least one source texel
    // beyond the most distant occupied footprint column.
    let tangent = f64::from(placed_fixture::TRACE_FOV * 0.5).tan();
    let mut occupied = 0;
    let mut clear = 0;
    for y in (1..side - 1).step_by(2) {
        for x in (1..side - 1).step_by(2) {
            let u = (x as f64 + 0.5) / side as f64;
            let v = (y as f64 + 0.5) / side as f64;
            let ray = [(1.0 - 2.0 * u) * tangent, 1.0, (2.0 * v - 1.0) * tangent];
            let norm = ray
                .into_iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            let at = (y * side + x) * 8 + 6;
            let drawn = -fp16(&bytes[at..at + 2]).ln();
            let sample = Sample {
                tick: fixture.state.tick(),
                origin: [0.0, 40.0, 0.0],
                direction: ray.map(|value| value / norm),
                length: 200.0 * norm,
                steps: 160,
            };
            let depth = field.optical_depth(
                &sample,
                &fixture.state,
                Density::Uneroded,
                |_| Some(&fixture.shape),
                weight,
            );
            if depth > 0.24 {
                assert!(
                    drawn > 0.04,
                    "weight {weight} missed a known body at {x}, {y}"
                );
                occupied += 1;
            }
            // The fixture's occupied z range is [-40, 40], no occupied
            // column starts below y=100, and a source texel spans 20 metres.
            if (60.0 * ray[2]).abs() > 60.0 {
                assert!(
                    drawn < 0.002,
                    "weight {weight} drew in clear air at {x}, {y}"
                );
                clear += 1;
            }
        }
    }
    assert!(occupied > 8 && clear > 8, "fixture must witness both cases");
}

#[cfg(target_os = "macos")]
fn fp16(bytes: &[u8]) -> f64 {
    let bits = u16::from_le_bytes(bytes.try_into().expect("fp16"));
    let exp = i32::from((bits >> 10) & 31);
    let fraction = f64::from(bits & 1023) / 1024.0;
    if exp == 0 {
        fraction * 2.0_f64.powi(-14)
    } else {
        (1.0 + fraction) * 2.0_f64.powi(exp - 15)
    }
}
