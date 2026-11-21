//! GPU time is bucketed by the category each pass declares. Every pass of
//! every compiled 3D scene variant lands in exactly one bucket, the profiler's
//! named buckets hold the passes the old name lists held, and the timing
//! module knows nothing of the passes it times.

mod registry_support;

use std::sync::Arc;
use thrum_render::registry_audit;
use thrum_render::timing::{PassTiming, SceneGpuTimings, TimingCategory};

fn timed(graph: &thrum_render::graph::CompiledGraph) -> Vec<PassTiming> {
    // A distinct power of two per pass, so a pass counted twice or dropped
    // shows in every sum it touches.
    graph
        .pass_timings()
        .enumerate()
        .map(|(index, (name, category))| PassTiming {
            name: Arc::from(name),
            category,
            nanoseconds: f64::from(1_u32 << (index % 31)),
        })
        .collect()
}

fn sum(passes: &[PassTiming], member: impl Fn(&str) -> bool) -> f64 {
    passes
        .iter()
        .filter(|pass| member(&pass.name))
        .map(|pass| pass.nanoseconds)
        .sum()
}

// The bucket membership the profiler read before categories, by name. A
// pass the old lists did not name was in no named bucket.
const CULLING: [&str; 5] = [
    "camera visibility",
    "camera prepass list",
    "camera depth prepass",
    "camera depth pyramid",
    "camera final cull",
];

fn named_buckets(timings: &SceneGpuTimings) -> [(&'static str, f64); 12] {
    [
        ("shadow near", timings.shadow_near()),
        ("shadow far", timings.shadow_far()),
        ("clusters", timings.clusters()),
        ("forward", timings.forward()),
        ("history", timings.history()),
        ("tone", timings.tone()),
        ("atmosphere", timings.atmosphere()),
        ("transmittance", timings.transmittance()),
        ("multiple ground", timings.multiple_ground()),
        ("sky", timings.sky()),
        ("diffuse", timings.diffuse()),
        ("aerial", timings.aerial()),
    ]
}

fn name_buckets(passes: &[PassTiming]) -> [(&'static str, f64); 12] {
    [
        (
            "shadow near",
            sum(passes, |n| {
                matches!(n, "synthetic Sun shadow near" | "synthetic Sun caster cull")
            }),
        ),
        (
            "shadow far",
            sum(passes, |n| {
                matches!(
                    n,
                    "synthetic Sun shadow far"
                        | "synthetic Sun horizon heights"
                        | "synthetic Sun air shadow heights"
                )
            }),
        ),
        ("clusters", sum(passes, |n| n == "synthetic cluster build")),
        (
            "forward",
            sum(passes, |n| {
                matches!(
                    n,
                    "synthetic forward opaque" | "contribution compute" | "atmosphere stars"
                ) || CULLING.contains(&n)
            }),
        ),
        (
            "history",
            sum(passes, |n| n == "synthetic transform history"),
        ),
        (
            "tone",
            sum(passes, |n| {
                matches!(n, "synthetic exposure resolve" | "synthetic tone encode")
            }),
        ),
        (
            "atmosphere",
            sum(passes, |n| {
                n.starts_with("atmosphere ") && n != "atmosphere stars"
            }),
        ),
        (
            "transmittance",
            sum(passes, |n| n == "atmosphere transmittance"),
        ),
        (
            "multiple ground",
            sum(passes, |n| {
                matches!(
                    n,
                    "atmosphere multiple scattering" | "atmosphere ground irradiance"
                )
            }),
        ),
        (
            "sky",
            sum(passes, |n| {
                n.starts_with("atmosphere sky ")
                    || n.starts_with("atmosphere reflection ")
                    || n == "atmosphere DFG integration"
            }),
        ),
        (
            "diffuse",
            sum(passes, |n| n == "atmosphere diffuse irradiance"),
        ),
        (
            "aerial",
            sum(passes, |n| n == "atmosphere aerial perspective"),
        ),
    ]
}

#[test]
fn every_pass_lands_in_one_bucket_and_the_profiler_buckets_are_unchanged() {
    let Some(variants) = registry_support::variants("timing categories") else {
        return;
    };
    let mut categories = std::collections::BTreeSet::new();
    for variant in &variants {
        let label = &variant.label;
        let passes = timed(&variant.graph);
        let timings = registry_audit::scene_timings(&passes);
        // Every timed pass has a timestamp slot, so none can read as zero.
        assert!(
            usize::try_from(variant.scene_pass_count)
                .is_ok_and(|slots| slots >= registry_audit::pipeline_pass_count(&variant.graph)),
            "{label}: fewer timestamp slots than timed passes"
        );
        let total: f64 = passes.iter().map(|pass| pass.nanoseconds).sum();
        let bucketed: f64 = TimingCategory::ALL
            .iter()
            .map(|&category| timings.bucket(category))
            .sum();
        assert_eq!(bucketed, total, "{label}: a pass fell outside every bucket");
        for pass in &passes {
            assert!(
                TimingCategory::ALL.contains(&pass.category),
                "{label}: {} declared no reported bucket",
                pass.name
            );
            categories.insert(pass.category);
        }
        assert_eq!(
            named_buckets(&timings),
            name_buckets(&passes),
            "{label}: a profiler bucket changed membership"
        );
        assert_eq!(
            timings.sum(),
            sum(&passes, |n| n != "V1 surface present"),
            "{label}: the scene sum changed"
        );
        let names: Vec<_> = timings.values().into_keys().map(str::to_owned).collect();
        let mut expected: Vec<_> = variant.graph.pass_names().map(str::to_owned).collect();
        expected.sort();
        assert_eq!(names, expected, "{label}: the profiler rows changed");
    }
    for category in [
        TimingCategory::ShadowNear,
        TimingCategory::ShadowFar,
        TimingCategory::Clusters,
        TimingCategory::Forward,
        TimingCategory::History,
        TimingCategory::Tone,
        TimingCategory::AtmosphereTransmittance,
        TimingCategory::AtmosphereMultipleGround,
        TimingCategory::AtmosphereSky,
        TimingCategory::AtmosphereDiffuse,
        TimingCategory::AtmosphereAerial,
        TimingCategory::Temporal,
    ] {
        assert!(
            categories.contains(&category),
            "no variant timed {category:?}"
        );
    }
}

#[test]
fn a_renamed_pass_keeps_its_bucket() {
    let pass = |name: &str| PassTiming {
        name: Arc::from(name),
        category: TimingCategory::ShadowFar,
        nanoseconds: 5.0,
    };
    let before = registry_audit::scene_timings(&[pass("synthetic Sun horizon heights")]);
    let after = registry_audit::scene_timings(&[pass("Sun horizon height field")]);
    assert_eq!(before.shadow_far(), 5.0);
    assert_eq!(after.shadow_far(), before.shadow_far());
    assert_eq!(after.sum(), before.sum());
}

#[test]
fn the_timing_module_names_no_pass_and_imports_no_renderer_module() {
    let source = include_str!("../src/timing.rs");
    for owner in [
        "crate::synthetic",
        "crate::atmosphere",
        "crate::present",
        "crate::visibility",
        "crate::sprite",
        "crate::pass_key",
    ] {
        assert!(!source.contains(owner), "timing.rs reaches into {owner}");
    }
    for name in ["\"synthetic ", "\"atmosphere ", "\"camera ", "\"V1 surface"] {
        assert!(
            !source.contains(name),
            "timing.rs matches a pass name {name}"
        );
    }
}
