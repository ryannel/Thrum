use std::collections::{BTreeMap, BTreeSet};

use super::{AtmosphereStage, PassKey, PipelineRole, SecondaryStage};
use crate::timing::TimingCategory;

/// Contributed passes of one kind the role ranges hold apart; the tests prove
/// the ranges do not meet below this count.
const MAX_CONTRIBUTED: u32 = 1000;
/// Secondary views the role ranges hold apart; the tests prove the secondary
/// range stays below the tagged roles at this count.
const MAX_SECONDARY_VIEWS: u32 = 1000;

/// Every key with one name in every graph.
pub(crate) fn fixed() -> impl Iterator<Item = PassKey> {
    [
        PassKey::Opaque,
        PassKey::Display,
        PassKey::DisplayReferred,
        PassKey::ShadowNear,
        PassKey::ShadowFar,
        PassKey::HorizonHeights,
        PassKey::AirShadow,
        PassKey::CasterCull,
        PassKey::Cluster,
        PassKey::History,
        PassKey::Exposure,
        PassKey::Stars,
        PassKey::CameraVisibility,
        PassKey::PrepassList,
        PassKey::DepthPrepass,
        PassKey::DepthPyramid,
        PassKey::FinalCull,
        PassKey::CullingStats,
        PassKey::TemporalExposure,
        PassKey::TemporalWitness,
        PassKey::Taau,
        PassKey::ContributionUpload,
        PassKey::ContributionCompute,
        PassKey::AfterOpaqueCopy,
        PassKey::RayStructureBuild,
        PassKey::NativeReset,
        PassKey::NativeWitness,
        PassKey::Sprites,
        PassKey::Readback,
        PassKey::Surface,
    ]
    .into_iter()
    .chain(AtmosphereStage::ORDER.map(PassKey::Atmosphere))
}

/// The category the renderer gives a pass of this name, for tests that
/// build timings by name. Contributed passes are named "effect …".
pub(crate) fn timing_of(name: &str) -> TimingCategory {
    fixed()
        .find(|key| key.fixed_name() == Some(name))
        .map_or_else(
            || {
                if name.starts_with("effect ") {
                    TimingCategory::Effects
                } else {
                    TimingCategory::Transfer
                }
            },
            PassKey::timing,
        )
}

fn indexed_keys() -> impl Iterator<Item = PassKey> {
    let limit = MAX_CONTRIBUTED - 1;
    let views = MAX_SECONDARY_VIEWS - 1;
    [0, 1, limit]
        .into_iter()
        .flat_map(|index| [PassKey::Effect(index), PassKey::AfterOpaque(index)])
        .chain([0, 1, views].into_iter().flat_map(|view| {
            [
                SecondaryStage::Opaque,
                SecondaryStage::Clusters,
                SecondaryStage::Aerial,
                SecondaryStage::SunNear,
                SecondaryStage::SunFar,
                SecondaryStage::Visibility,
                SecondaryStage::Casters,
            ]
            .map(|stage| PassKey::Secondary { view, stage })
        }))
}

#[test]
fn every_pipeline_role_value_belongs_to_one_role() {
    // Roles shared on purpose: the 2D sprite draw and the 3D opaque draw never
    // share a graph, and the prepass list reuses camera visibility's pipeline.
    let mut owners = BTreeMap::<u32, BTreeSet<String>>::new();
    for key in fixed().chain(indexed_keys()) {
        if let Some(role) = key.pipeline_role() {
            owners
                .entry(role.value())
                .or_default()
                .insert(format!("{role:?}"));
        }
    }
    for (value, roles) in &owners {
        let shared = roles.len() == 1
            || *roles == BTreeSet::from(["Opaque".to_owned(), "Sprites".to_owned()]);
        assert!(shared, "role value {value} is claimed by {roles:?}");
    }
}

#[test]
fn indexed_role_ranges_stay_apart_up_to_their_bounds() {
    let tagged = [
        PipelineRole::DepthPrepass,
        PipelineRole::TemporalExposure,
        PipelineRole::Surface,
        PipelineRole::CasterCull,
    ]
    .map(PipelineRole::value);
    let lowest_tag = tagged.into_iter().min().unwrap_or(0);
    let last_secondary = PipelineRole::Secondary {
        view: MAX_SECONDARY_VIEWS - 1,
        stage: SecondaryStage::Casters,
    }
    .value();
    assert!(last_secondary < lowest_tag);
    let last_effect = PipelineRole::Effect(MAX_CONTRIBUTED - 1).value();
    assert!(last_effect < PipelineRole::AfterOpaque(0).value());
    let last_after = PipelineRole::AfterOpaque(MAX_CONTRIBUTED - 1).value();
    assert!(
        last_after
            < PipelineRole::Secondary {
                view: 0,
                stage: SecondaryStage::Opaque,
            }
            .value()
    );
    let last_atmosphere = PipelineRole::Atmosphere(AtmosphereStage::Reflection(6)).value();
    assert!(last_atmosphere < PipelineRole::ContributionCompute.value());
}

#[test]
fn every_named_key_has_a_unique_name_and_contributed_keys_have_none() {
    let mut names = BTreeSet::new();
    for key in fixed().chain(indexed_keys()) {
        match key {
            PassKey::Effect(_) | PassKey::AfterOpaque(_) => assert!(key.name().is_none()),
            _ => {
                let name = key.name().expect("named key").into_owned();
                assert!(names.insert(name.clone()), "{key:?} repeats {name}");
            }
        }
    }
}

#[test]
fn the_test_name_lookup_returns_each_fixed_key_bucket() {
    for key in fixed() {
        let name = key.constant_name();
        assert_eq!(timing_of(name), key.timing(), "{name}");
    }
}
