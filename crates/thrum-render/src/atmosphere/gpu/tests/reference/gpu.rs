//! A complete matched-medium observation grid. This reports disagreement and
//! sampling uncertainty; passing it does not qualify physical sky accuracy.

mod query;
mod readback;

use std::fmt::Write;

use super::{Case, Single, Stats, Unresolved, deterministic_single, estimate};

const SUN_DEGREES: [i16; 6] = [45, 0, -3, -6, -9, -12];
const HEIGHT: f64 = 2.0;

fn views() -> [(f64, f64); 11] {
    let horizon = -(super::geometry::RADIUS / (super::geometry::RADIUS + HEIGHT))
        .acos()
        .to_degrees();
    [
        (90.0, 0.0),
        (15.0, 0.0),
        (15.0, 90.0),
        (15.0, 180.0),
        (1.0, 0.0),
        (1.0, 90.0),
        (1.0, 180.0),
        (-1.0, 0.0),
        (-15.0, 0.0),
        (horizon - 0.005, 0.0),
        (horizon + 0.005, 0.0),
    ]
}

fn row(
    case: Case,
    seed: u64,
    gpu: [f64; 2],
    single: Result<Single, Unresolved>,
    mc: Result<Stats, Unresolved>,
) -> String {
    let mut result = format!(
        "{}\t{:.8}\t{:.8}\t{:.8}\t{seed}\t{:.17e}\t{:.17e}",
        case.band,
        case.sun_elevation.to_degrees(),
        case.view_elevation.to_degrees(),
        case.relative_azimuth.to_degrees(),
        gpu[0],
        gpu[1]
    );
    match single {
        Ok(value) => write!(
            result,
            "\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\tcomplete",
            value.volume, value.unscattered_ground, value.radiance, value.estimated_error
        )
        .expect("format reference report into String"),
        Err(error) => write!(result, "\t\t\t\t\t{:?}", error.reason)
            .expect("format reference report into String"),
    }
    match mc {
        Ok(value) => {
            let status = if value.nonzero_paths == 0 {
                "zero_samples_unresolved"
            } else if value.zero_batches > 0 {
                "sparse_batches_estimate_only"
            } else {
                "batch_estimate_only"
            };
            write!(
                result,
                "\t{:.17e}\t{:.17e}\t{}\t{}\t{}\t{status}",
                value.radiance.mean,
                value.radiance.standard_error,
                value.paths,
                value.nonzero_paths,
                value.zero_batches
            )
            .expect("format reference report into String");
            for term in [
                value.direct_volume,
                value.direct_ground,
                value.atmosphere,
                value.ground,
                value.ground_vertices,
            ]
            .into_iter()
            .chain(value.by_order)
            {
                write!(result, "\t{:.17e}\t{:.17e}", term.mean, term.standard_error)
                    .expect("format reference report into String");
            }
            write!(
                result,
                "\t{:.17e}\t{:.17e}\t{}\t{}\t{:?}\t{:?}",
                value.largest_path,
                value.largest_event,
                value.max_interactions,
                value.seed,
                value.nonzero_by_order,
                value.batch_means
            )
            .expect("format reference report into String");
        }
        Err(error) => {
            write!(
                result,
                "\t\t\t{}\t\t\t{:?}:completed={}",
                error.requested_paths, error.reason, error.completed_paths
            )
            .expect("format reference report into String");
            for _ in 0..26 {
                result.push('\t');
            }
        }
    }
    result.push('\n');
    result
}

#[test]
fn gpu_transport_reference_reports_every_prescribed_row() {
    pollster::block_on(async {
        let paths = std::env::var("THRUM_G3S_REFERENCE_PATHS").map_or(1024, |value| {
            value
                .parse::<usize>()
                .expect("integer reference path count")
        });
        assert!(paths > 0 && paths.is_multiple_of(8));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for atmosphere reference comparison");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the observer sends physical directions to the f32 GPU shader"
        )]
        let directions = views().map(|(view, azimuth)| {
            let (sv, cv) = view.to_radians().sin_cos();
            let (sa, ca) = azimuth.to_radians().sin_cos();
            [(cv * ca) as f32, sv as f32, (cv * sa) as f32, 0.0]
        });
        let mut probe = readback::Probe::new(&device, &queue, &directions);
        let mut report = String::from(
            "# Matched declared coefficient bands, unit source; L/E0 in sr^-1. No CIE/lux/Earth accuracy claim.\n# Batch SE and quadrature estimates are not rigorous bounds. Zero/sparse batches do not prove darkness.\n",
        );
        report.push_str("band\tsun_deg\tview_deg\tazimuth_deg\tseed\tgpu_full\tgpu_single\tquad_volume\tquad_ground\tquad_total\tquad_error\tquad_status\tmc_mean\tmc_se\tpaths\tnonzero_paths\tzero_batches\tmc_status");
        for name in [
            "direct_volume",
            "direct_ground",
            "never_ground",
            "touched_ground",
            "ground_vertex",
            "atmo_order1",
            "atmo_order2",
            "atmo_order3",
            "atmo_order4",
            "atmo_order5plus",
        ] {
            write!(report, "\t{name}\t{name}_se").unwrap();
        }
        report.push_str("\tlargest_path\tlargest_event\tmax_interactions\tmc_seed\tnonzero_orders\tbatch_means\n");
        let mut unresolved = 0;
        let mut count = 0;
        for (sun_id, sun) in SUN_DEGREES.into_iter().enumerate() {
            let observed = probe.sky(&device, &queue, sun);
            let (full, single) = (&observed.full, &observed.single);
            assert_eq!(full.len(), single.len());
            for (full, single) in full.iter().zip(single) {
                for channel in 0..3 {
                    assert!(
                        full[channel] >= single[channel],
                        "nonnegative multiple scattering"
                    );
                }
            }
            for (view_id, (view, azimuth)) in views().into_iter().enumerate() {
                let actual = observed.rays_full[view_id];
                let direct = observed.rays_single[view_id];
                for band in 0..3 {
                    let case = Case::degrees(band, f64::from(sun), view, azimuth, HEIGHT);
                    // Fixed semantic tuple slots; adding rays cannot change earlier seeds.
                    let seed = 0x7468_7275_6d73_6b79
                        ^ u64::try_from((sun_id << 8) | (view_id << 3) | band).unwrap();
                    let quadrature = deterministic_single(case);
                    let mc = estimate(case, paths, seed);
                    unresolved += usize::from(quadrature.is_err() || mc.is_err());
                    report.push_str(&row(
                        case,
                        seed,
                        [actual[band], direct[band]],
                        quadrature,
                        mc,
                    ));
                    count += 1;
                }
            }
        }
        assert_eq!(count, 198);
        if let Some(path) = std::env::var_os("THRUM_G3S_REFERENCE_OUTPUT") {
            std::fs::write(path, report).expect("write explicit reference observations");
        }
        assert!(scope.pop().await.is_none());
        assert_eq!(
            unresolved, 0,
            "reference evaluation incomplete; inspect every retained row"
        );
    });
}
