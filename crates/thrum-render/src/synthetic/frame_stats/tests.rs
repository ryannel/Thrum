use super::*;
use crate::timing::PassTiming;

fn timings(atmosphere: &[&str]) -> Vec<PassTiming> {
    atmosphere
        .iter()
        .copied()
        .chain([
            "synthetic forward opaque",
            "synthetic transform history",
            "synthetic exposure resolve",
            "synthetic tone encode",
        ])
        .map(|name| PassTiming {
            name: name.into(),
            nanoseconds: 1.0,
        })
        .collect()
}

#[test]
fn cold_and_cached_ground_queries_keep_the_report_shape_and_profile_bucket() {
    let cold = timings(&[
        "atmosphere transmittance",
        "atmosphere multiple scattering",
        "atmosphere ground irradiance",
        "atmosphere sky view",
        "atmosphere diffuse irradiance",
        "atmosphere aerial perspective",
    ]);
    let (values, present, _) = collect_pass_durations(&cold, false, false, false).unwrap();
    assert_eq!(&values[6..], &[1.0, 2.0, 1.0, 1.0, 1.0]);
    assert_eq!(values[5], 2.0, "exposure resolve remains in display bucket");
    assert!(present.is_none());
    let cached = timings(&[
        "atmosphere sky view",
        "atmosphere diffuse irradiance",
        "atmosphere aerial perspective",
    ]);
    let (values, _, _) = collect_pass_durations(&cached, false, false, false).unwrap();
    assert_eq!(&values[6..], &[0.0, 0.0, 1.0, 1.0, 1.0]);
    let ground_only = timings(&["atmosphere ground irradiance"]);
    let (values, _, _) = collect_pass_durations(&ground_only, false, false, false).unwrap();
    assert_eq!(&values[6..], &[0.0, 1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn reflection_queries_keep_raw_identity_and_the_existing_sky_bucket() {
    let names =
        crate::atmosphere::TIMING_ORDER.map(|(stage, _)| crate::atmosphere::STAGE_NAMES[stage]);
    let cold = timings(&names);
    let (values, _, _) = collect_pass_durations(&cold, false, false, false).unwrap();
    assert_eq!(&values[6..], &[1.0, 2.0, 9.0, 1.0, 1.0]);
    let cached = timings(&names[7..]);
    let (values, _, _) = collect_pass_durations(&cached, false, false, false).unwrap();
    assert_eq!(&values[6..], &[0.0, 0.0, 7.0, 0.0, 0.0]);
    let unexpected = timings(&["atmosphere reflection mip 99"]);
    assert!(collect_pass_durations(&unexpected, false, false, false).is_none());
}

#[test]
fn a_contribution_compute_pass_is_reported_apart_from_the_scene_sequence() {
    let mut passes = timings(&["atmosphere sky view"]);
    passes.insert(
        1,
        PassTiming {
            name: super::super::CONTRIBUTION_COMPUTE.into(),
            nanoseconds: 3.0,
        },
    );
    let (values, _, contribution) = collect_pass_durations(&passes, false, false, false).unwrap();
    assert_eq!(contribution, Some(3.0));
    assert_eq!(
        values[3], 4.0,
        "forward includes the contribution so totals do"
    );
    let (_, _, none) = collect_pass_durations(&timings(&[]), false, false, false).unwrap();
    assert!(none.is_none());
    passes.push(passes[1].clone());
    assert!(collect_pass_durations(&passes, false, false, false).is_none());
}

#[test]
fn the_horizon_height_pass_reports_in_the_far_shadow_bucket() {
    let passes = timings(&[
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic Sun horizon heights",
    ]);
    let (values, _, _) = collect_pass_durations(&passes, false, true, false).unwrap();
    assert_eq!(&values[..2], &[1.0, 2.0]);
    let missing = timings(&["synthetic Sun shadow near", "synthetic Sun shadow far"]);
    assert!(collect_pass_durations(&missing, false, true, false).is_none());
}
