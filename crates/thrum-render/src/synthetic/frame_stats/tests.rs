use super::*;
use crate::timing::PassTiming;

const TRANSFORMS: SceneTimingLayout = SceneTimingLayout {
    transforms: true,
    star_pass: None,
    shadows: Shadows::Off,
    clusters: false,
    culling: culling_passes(CullMode::Direct),
};

const CULLED: SceneTimingLayout = SceneTimingLayout {
    culling: culling_passes(CullMode::Frustum),
    ..TRANSFORMS
};

fn timings(atmosphere: &[&str]) -> Vec<PassTiming> {
    atmosphere
        .iter()
        .copied()
        .chain([
            "synthetic exposure resolve",
            "synthetic forward opaque",
            "synthetic transform history",
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
    let (values, present, _) = pass_durations(&cold, false, TRANSFORMS).unwrap();
    assert_eq!(
        &[
            values.transmittance(),
            values.multiple_ground(),
            values.sky(),
            values.diffuse(),
            values.aerial()
        ],
        &[1.0, 2.0, 1.0, 1.0, 1.0]
    );
    assert_eq!(
        values.tone(),
        2.0,
        "exposure resolve remains in display bucket"
    );
    assert!(present.is_none());
    let cached = timings(&[
        "atmosphere sky view",
        "atmosphere diffuse irradiance",
        "atmosphere aerial perspective",
    ]);
    let (values, _, _) = pass_durations(&cached, false, TRANSFORMS).unwrap();
    assert_eq!(
        &[
            values.transmittance(),
            values.multiple_ground(),
            values.sky(),
            values.diffuse(),
            values.aerial()
        ],
        &[0.0, 0.0, 1.0, 1.0, 1.0]
    );
    let ground_only = timings(&["atmosphere ground irradiance"]);
    let (values, _, _) = pass_durations(&ground_only, false, TRANSFORMS).unwrap();
    assert_eq!(
        &[
            values.transmittance(),
            values.multiple_ground(),
            values.sky(),
            values.diffuse(),
            values.aerial()
        ],
        &[0.0, 1.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn reflection_queries_keep_raw_identity_and_the_existing_sky_bucket() {
    let names =
        crate::atmosphere::TIMING_ORDER.map(|(stage, _)| crate::atmosphere::STAGE_NAMES[stage]);
    let cold = timings(&names);
    let (values, _, _) = pass_durations(&cold, false, TRANSFORMS).unwrap();
    assert_eq!(
        &[
            values.transmittance(),
            values.multiple_ground(),
            values.sky(),
            values.diffuse(),
            values.aerial()
        ],
        &[1.0, 2.0, 9.0, 1.0, 1.0]
    );
    let cached = timings(&names[7..]);
    let (values, _, _) = pass_durations(&cached, false, TRANSFORMS).unwrap();
    assert_eq!(
        &[
            values.transmittance(),
            values.multiple_ground(),
            values.sky(),
            values.diffuse(),
            values.aerial()
        ],
        &[0.0, 0.0, 7.0, 0.0, 0.0]
    );
    let unexpected = timings(&["atmosphere reflection mip 99"]);
    assert!(pass_durations(&unexpected, false, TRANSFORMS).is_none());
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
    let (values, _, contribution) = pass_durations(&passes, false, TRANSFORMS).unwrap();
    assert_eq!(contribution, Some(3.0));
    assert_eq!(
        values.forward(),
        4.0,
        "forward includes the contribution so totals do"
    );
    let (_, _, none) = pass_durations(&timings(&[]), false, TRANSFORMS).unwrap();
    assert!(none.is_none());
    passes.push(passes[1].clone());
    assert!(pass_durations(&passes, false, TRANSFORMS).is_none());
}

#[test]
fn the_horizon_height_pass_reports_in_the_far_shadow_bucket() {
    let passes = timings(&[
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic Sun horizon heights",
    ]);
    let (values, _, _) = pass_durations(
        &passes,
        false,
        SceneTimingLayout {
            shadows: Shadows::Unculled,
            ..TRANSFORMS
        },
    )
    .unwrap();
    assert_eq!(&[values.shadow_near(), values.shadow_far()], &[1.0, 2.0]);
    let missing = timings(&["synthetic Sun shadow near", "synthetic Sun shadow far"]);
    assert!(
        pass_durations(
            &missing,
            false,
            SceneTimingLayout {
                shadows: Shadows::Unculled,
                ..TRANSFORMS
            }
        )
        .is_none()
    );
}

#[test]
fn camera_visibility_is_required_by_the_culled_layout_and_counted_in_forward() {
    let mut passes = timings(&["atmosphere sky view"]);
    passes.insert(
        1,
        PassTiming {
            name: crate::visibility::PASS.into(),
            nanoseconds: 2.0,
        },
    );
    let (values, _, contribution) = pass_durations(&passes, false, CULLED).unwrap();
    assert_eq!(values.forward(), 3.0, "forward includes camera visibility");
    assert!(contribution.is_none());
    // The pass is exact to the slot layout: absent when culled, present when direct, or twice.
    assert!(pass_durations(&timings(&["atmosphere sky view"]), false, CULLED).is_none());
    assert!(pass_durations(&passes, false, TRANSFORMS).is_none());
    passes.push(passes[1].clone());
    assert!(pass_durations(&passes, false, CULLED).is_none());
}

#[test]
fn occlusion_passes_are_required_once_each_and_counted_in_forward() {
    const OCCLUDED: SceneTimingLayout = SceneTimingLayout {
        culling: culling_passes(CullMode::Occlusion),
        ..TRANSFORMS
    };
    let mut passes = timings(&["atmosphere sky view"]);
    let culling = [
        crate::visibility::PREPASS_LIST_PASS,
        crate::synthetic::camera_culling::DEPTH_PREPASS_PASS,
        crate::visibility::PYRAMID_PASS,
        crate::visibility::FINAL_PASS,
    ];
    for (offset, name) in culling.into_iter().enumerate() {
        passes.insert(
            2 + offset,
            PassTiming {
                name: name.into(),
                nanoseconds: 2.0,
            },
        );
    }
    let (values, _, _) = pass_durations(&passes, false, OCCLUDED).unwrap();
    assert_eq!(
        values.forward(),
        9.0,
        "forward includes all four culling passes"
    );
    // Exact to the layout: refused under Frustum or Direct, with one missing, or with a repeat.
    assert!(pass_durations(&passes, false, CULLED).is_none());
    assert!(pass_durations(&passes, false, TRANSFORMS).is_none());
    let mut night = passes.clone();
    night.push(PassTiming {
        name: super::super::stars::PASS_NAME.into(),
        nanoseconds: 3.0,
    });
    let night_layout = SceneTimingLayout {
        star_pass: Some(super::super::stars::PASS_NAME),
        ..OCCLUDED
    };
    assert!(pass_durations(&night, false, OCCLUDED).is_none());
    assert!(pass_durations(&passes, false, night_layout).is_none());
    let (values, _, _) = pass_durations(&night, false, night_layout).unwrap();
    assert_eq!(values.forward(), 12.0, "forward includes culling and stars");

    let mut missing = passes.clone();
    assert_eq!(
        missing.remove(4).name.as_ref(),
        crate::visibility::PYRAMID_PASS
    );
    assert!(pass_durations(&missing, false, OCCLUDED).is_none());
    let mut repeated = passes.clone();
    repeated.push(passes[2].clone());
    assert!(pass_durations(&repeated, false, OCCLUDED).is_none());
}

#[test]
fn the_caster_cull_is_required_once_and_counted_in_the_near_shadow_bucket() {
    let layout = SceneTimingLayout {
        shadows: Shadows::Culled,
        ..CULLED
    };
    let named = |names: &[&str]| -> Vec<PassTiming> {
        names
            .iter()
            .map(|&name| PassTiming {
                name: name.into(),
                nanoseconds: 1.0,
            })
            .collect()
    };
    let scene = [
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic Sun horizon heights",
        "synthetic exposure resolve",
        "synthetic forward opaque",
        "synthetic transform history",
        "synthetic tone encode",
    ];
    let mut passes = vec![crate::visibility::casters::PASS, crate::visibility::PASS];
    passes.extend(scene);
    let (values, _, _) = pass_durations(&named(&passes), false, layout).unwrap();
    assert_eq!(values.shadow_near(), 2.0, "the cull joins the near map");
    assert_eq!(values.shadow_far(), 2.0);
    assert_eq!(values.forward(), 2.0, "camera visibility stays in forward");
    let mut missing = vec![crate::visibility::PASS];
    missing.extend(scene);
    assert!(pass_durations(&named(&missing), false, layout).is_none());
    let mut twice = passes.clone();
    twice.insert(0, crate::visibility::casters::PASS);
    assert!(pass_durations(&named(&twice), false, layout).is_none());
    let unexpected = SceneTimingLayout {
        shadows: Shadows::Unculled,
        ..layout
    };
    assert!(pass_durations(&named(&passes), false, unexpected).is_none());
}
