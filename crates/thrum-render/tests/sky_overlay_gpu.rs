//! Hardware proof of cloud-free lending and filtered physical sky products.
mod support;
mod world_light_fixture;

use thrum_render::atmosphere::AtmosphereFrame;
use world_light_fixture::{BINDINGS, prepare, probe};

const SCALE: f64 = 1024.0;

fn constant(_: u32, _: u32, _: u32, _: u32) -> [f32; 4] {
    [1024.0, 1536.0, 2048.0, 0.0]
}

fn half(face: u32, x: u32, _: u32, edge: u32) -> [f32; 4] {
    let west = match face {
        0 => false,
        1 => true,
        5 => x >= edge / 2,
        _ => x < edge / 2,
    };
    if west {
        constant(0, 0, 0, 0)
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

fn source(west: bool, clear: [f32; 4]) -> [f64; 3] {
    if west {
        [1024.0, 1536.0, 2048.0]
    } else {
        std::array::from_fn(|channel| f64::from(clear[channel]))
    }
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "unit quadrature directions narrow at the shader boundary"
)]
fn metal_filters_constant_and_half_sky_in_physical_units() {
    let Some(gpu) = thrum_test_support::open(
        "sky overlay",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    assert_eq!(gpu.gpu.adapter.get_info().backend, wgpu::Backend::Metal);
    pollster::block_on(async {
        let frame =
            AtmosphereFrame::new([0.0, 1.0, 0.0], 10_000.0, [1.0; 3], 40.0, 1).expect("daylight");
        let (base, no_overlay) = prepare(&gpu, frame, None).await;
        let (repeat, repeated) = prepare(&gpu, frame, None).await;
        assert_eq!(base.shader_source, repeat.shader_source);
        assert_eq!(no_overlay, repeated);

        let rows = 128;
        let sectors = 256;
        let directions: Vec<_> = (0..rows)
            .flat_map(|row| {
                (0..sectors).map(move |sector| {
                    let mu = -1.0 + 2.0 * (f64::from(row) + 0.5) / f64::from(rows);
                    let phi =
                        2.0 * std::f64::consts::PI * (f64::from(sector) + 0.5) / f64::from(sectors);
                    let radius = (1.0 - mu * mu).sqrt();
                    [
                        (radius * phi.cos()) as f32,
                        mu as f32,
                        (radius * phi.sin()) as f32,
                        0.0,
                    ]
                })
            })
            .collect();
        let read_sky = format!(
            "{BINDINGS}
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {{
    output[id.x] = vec4(environment_sky_radiance(queries[id.x].xyz),1.0);
}}"
        );
        let clear_sky = probe(&gpu.device, &gpu.queue, &base, &directions, &read_sky, 1);

        let mut normals = vec![
            [1.0, 0.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
        ];
        for x in [-1.0_f32, 1.0] {
            for y in [-1.0_f32, 1.0] {
                for z in [-1.0_f32, 1.0] {
                    normals.push([
                        x / 3.0_f32.sqrt(),
                        y / 3.0_f32.sqrt(),
                        z / 3.0_f32.sqrt(),
                        0.0,
                    ]);
                }
            }
        }
        let query = format!(
            "{BINDINGS}
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {{
    let direction = queries[id.x].xyz;
    let start = 4u*id.x;
    output[start] = vec4(environment_reflection(direction,0.045,0.5).radiance,1.0);
    output[start+1u] = vec4(environment_reflection(direction,1.0,0.5).radiance,1.0);
    output[start+2u] = vec4(environment_diffuse(direction),1.0);
    output[start+3u] = environment_light.horizontal_irradiance;
}}"
        );
        let baseline = probe(&gpu.device, &gpu.queue, &base, &normals, &query, 4);
        for is_half in [false, true] {
            let (env, _) = prepare(&gpu, frame, Some(if is_half { half } else { constant })).await;
            assert_eq!(
                probe(&gpu.device, &gpu.queue, &env, &normals, &read_sky, 1),
                probe(&gpu.device, &gpu.queue, &base, &normals, &read_sky, 1),
                "cloud lighting must see the clear sky"
            );
            let actual = probe(&gpu.device, &gpu.queue, &env, &normals, &query, 4);
            let mut m0 = [0.0; 3];
            let mut horizontal = [0.0; 3];
            let mut diffuse = vec![[0.0; 3]; normals.len()];
            let mut coefficients = [[0.0; 3]; 9];
            let solid_angle = 4.0 * std::f64::consts::PI / f64::from(rows * sectors);
            for (direction, clear) in directions.iter().zip(&clear_sky) {
                let [x, y, z, _] = direction.map(f64::from);
                let radiance = source(!is_half || x < 0.0, *clear);
                let basis = [
                    0.25,
                    x * 0.5,
                    y * 0.5,
                    z * 0.5,
                    (x * x - 1.0 / 3.0) * 15.0 / 32.0,
                    (y * y - 1.0 / 3.0) * 15.0 / 32.0,
                    x * y * 15.0 / 16.0,
                    x * z * 15.0 / 16.0,
                    y * z * 15.0 / 16.0,
                ];
                for c in 0..3 {
                    let flux = radiance[c] * solid_angle;
                    m0[c] += flux;
                    horizontal[c] += flux * y.max(0.0);
                    for (normal, irradiance) in normals.iter().zip(&mut diffuse) {
                        irradiance[c] += flux
                            * (x * f64::from(normal[0])
                                + y * f64::from(normal[1])
                                + z * f64::from(normal[2]))
                            .max(0.0);
                    }
                    for (coefficient, basis) in coefficients.iter_mut().zip(basis) {
                        coefficient[c] += flux * basis;
                    }
                }
            }
            let coeff_query = format!(
                "{BINDINGS}
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {{
    output[id.x] = environment_light.sky_diffuse[id.x] + queries[id.x];
}}"
            );
            let gpu_coefficients = probe(
                &gpu.device,
                &gpu.queue,
                &env,
                &[[0.0; 4]; 9],
                &coeff_query,
                1,
            );
            for (index, (reference, actual)) in
                coefficients.iter().zip(&gpu_coefficients).enumerate()
            {
                for c in 0..3 {
                    assert!(
                        (reference[c] - f64::from(actual[c])).abs() <= 0.01 * m0[c],
                        "P6 coefficients half={is_half} index={index} channel={c}: {actual:?} vs {reference:?}"
                    );
                }
            }
            for (i, normal) in normals.iter().enumerate() {
                for c in 0..3 {
                    let tolerance = (0.019 + 1.0 / 1024.0) * SCALE;
                    let reference = source(!is_half || normal[0] < 0.0, baseline[4 * i])[c];
                    if !is_half || normal[0].abs() > 0.1 {
                        assert!(
                            (f64::from(actual[4 * i][c]) - reference).abs() < tolerance,
                            "P7 half={is_half} normal={normal:?}: {:?} vs {reference}",
                            actual[4 * i]
                        );
                    }
                    // P7 bounds the widest mip at the six face axes, where
                    // the one-texel faces sample their own normal.
                    if i < 6 {
                        let wide = diffuse[i][c] / std::f64::consts::PI;
                        assert!(
                            (f64::from(actual[4 * i + 1][c]) - wide).abs() < tolerance,
                            "P7 widest mip half={is_half} normal={normal:?}: {:?} vs {wide}",
                            actual[4 * i + 1]
                        );
                    }
                    assert!(
                        (f64::from(actual[4 * i + 2][c]) - diffuse[i][c]).abs()
                            <= (3.0 / 32.0 + 0.01) * m0[c],
                        "H3 half={is_half} normal={normal:?}"
                    );
                    assert!((f64::from(actual[4 * i + 3][c]) - horizontal[c]).abs() < 0.01 * m0[c]);
                }
            }
            println!(
                "Metal: half-sky={is_half}, horizontal {horizontal:?}; 9 coefficients, 14 normals and lent reflection"
            );
        }
    });
}
