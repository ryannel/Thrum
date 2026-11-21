//! Every pass of every compiled 3D scene variant is declared by a typed key
//! that gives its name, its timing bucket and its pipeline role, and the
//! scene's timestamp slot count comes from the compiled graph.

mod registry_support;

use std::collections::{BTreeMap, BTreeSet};
use thrum_render::registry_audit;

#[test]
fn every_compiled_pass_carries_its_key_name_role_and_timing_bucket() {
    let Some(variants) = registry_support::variants("pass keys") else {
        return;
    };
    let mut seen = BTreeSet::new();
    for variant in &variants {
        let label = &variant.label;
        let passes = registry_audit::passes(&variant.graph);
        assert!(!passes.is_empty(), "{label}: empty graph");
        let mut keys = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut roles = BTreeSet::new();
        for pass in &passes {
            let name = &pass.name;
            let key = pass
                .key
                .as_ref()
                .unwrap_or_else(|| panic!("{label}: {name} was declared without a key"));
            assert!(keys.insert(key.clone()), "{label}: {key} declared twice");
            assert!(names.insert(name.clone()), "{label}: {name} named twice");
            if !pass.contributed {
                assert_eq!(
                    pass.key_name.as_deref(),
                    Some(name.as_str()),
                    "{label}: {key} is not named by its key"
                );
            }
            assert_eq!(
                pass.key_timing,
                Some(pass.timing),
                "{label}: {name}'s timing bucket is not its key's"
            );
            assert_eq!(
                pass.role, pass.key_role,
                "{label}: {name}'s pipeline role is not its key's"
            );
            if let Some(role) = pass.role {
                assert!(roles.insert(role), "{label}: role {role} used twice");
            }
            seen.insert(key.clone());
        }
    }
    // The matrix reaches the passes it is meant to: each culling mode,
    // both shadow arms, clusters, stars, temporal work and the readback.
    for key in [
        "CameraVisibility",
        "PrepassList",
        "DepthPrepass",
        "DepthPyramid",
        "FinalCull",
        "ShadowNear",
        "ShadowFar",
        "Cluster",
        "Stars",
        "Taau",
        "TemporalExposure",
        "Readback",
        "Atmosphere(Reflection(6))",
    ] {
        assert!(seen.contains(key), "no variant compiled {key}: {seen:?}");
    }
}

#[test]
fn the_scene_timestamp_slot_count_is_the_compiled_graph_count() {
    let Some(variants) = registry_support::variants("pass key slot count") else {
        return;
    };
    let mut largest = BTreeMap::<String, (u32, usize)>::new();
    for variant in &variants {
        // One prepared scene per label without its source and capture.
        let scene = variant
            .label
            .split(" source=")
            .next()
            .unwrap_or_default()
            .to_owned();
        let count = registry_audit::pipeline_pass_count(&variant.graph);
        let entry = largest
            .entry(scene)
            .or_insert((variant.scene_pass_count, 0));
        entry.1 = entry.1.max(count);
    }
    for (scene, (reported, compiled)) in largest {
        assert_eq!(
            usize::try_from(reported).ok(),
            Some(compiled),
            "{scene}: the prepared scene reports {reported} timed passes, its graphs hold {compiled}"
        );
    }
}
