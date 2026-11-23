//! Read-only views of the pass registry for integration tests: what each
//! compiled pass declared, and what its key says it should have declared.
//! Not part of the renderer's interface.

use crate::graph::{CompiledGraph, ResourceScope};
use crate::pass_key::{AtmosphereStage, PassKey, SecondaryStage};
use crate::timing::TimingCategory;

/// One live pass of a compiled graph.
#[derive(Clone, Debug)]
pub struct PassIdentity {
    /// The name the graph carries.
    pub name: String,
    /// The pass's key, printed; `None` for a pass declared without one.
    pub key: Option<String>,
    /// The name the key gives, if it names the pass itself.
    pub key_name: Option<String>,
    /// Whether the key is a contributed pass, named by its contribution.
    pub contributed: bool,
    /// The timing bucket the graph carries.
    pub timing: TimingCategory,
    /// The timing bucket the key gives.
    pub key_timing: Option<TimingCategory>,
    /// The pipeline role the graph carries.
    pub role: Option<u32>,
    /// The pipeline role the key gives.
    pub key_role: Option<u32>,
}

/// Every live pass of `graph`, in execution order.
pub fn passes(graph: &CompiledGraph) -> Vec<PassIdentity> {
    graph
        .pass_bindings()
        .map(|(_, pass)| PassIdentity {
            name: pass.name.to_string(),
            key: pass.key.map(|key| format!("{key:?}")),
            key_name: pass.key.and_then(PassKey::name).map(Into::into),
            contributed: matches!(pass.key, Some(PassKey::Effect(_) | PassKey::AfterOpaque(_))),
            timing: pass.timing,
            key_timing: pass.key.map(PassKey::timing),
            role: pass.pipeline.map(|pipeline| pipeline.role),
            key_role: pass
                .key
                .and_then(PassKey::pipeline_role)
                .map(crate::pass_key::PipelineRole::value),
        })
        .collect()
}

/// Live passes in `graph` that record with a pipeline and take timestamps.
pub fn pipeline_pass_count(graph: &CompiledGraph) -> usize {
    graph.pipeline_pass_count()
}

/// Metadata supplied by an atmosphere stage key before device preparation.
#[cfg(feature = "test-support")]
pub struct StageKeyAudit {
    /// The stage's pass name.
    pub name: &'static str,
    /// The charts and bindings in the stage table.
    pub table_reads: Vec<(String, u32)>,
    /// The stage's pipeline role.
    pub role: Option<u32>,
    /// The stage's cache bit.
    pub validity: u8,
}

/// Metadata for each atmosphere stage in declaration order.
#[cfg(feature = "test-support")]
pub fn atmosphere_stage_keys() -> impl Iterator<Item = StageKeyAudit> {
    AtmosphereStage::ORDER
        .into_iter()
        .map(|stage| StageKeyAudit {
            name: stage.name(),
            table_reads: stage_table_reads(stage),
            role: PassKey::Atmosphere(stage)
                .pipeline_role()
                .map(crate::pass_key::PipelineRole::value),
            validity: stage.validity(),
        })
}

fn stage_table_reads(stage: AtmosphereStage) -> Vec<(String, u32)> {
    stage
        .chart_reads()
        .iter()
        .map(|read| (format!("{:?}", read.chart), read.binding))
        .collect()
}

/// One atmosphere stage pass, its graph reads beside its table.
#[derive(Clone, Debug)]
pub struct StageAudit {
    /// The pass name.
    pub name: String,
    /// Charts the graph pass reads, in access order.
    pub graph_reads: Vec<String>,
    /// Charts and bindings the stage table lists; the bind group is built
    /// from this list.
    pub table_reads: Vec<(String, u32)>,
    /// The pipeline role the pass carries.
    pub role: Option<u32>,
    /// The role the stage key gives.
    pub key_role: u32,
    /// The stage's cache bit.
    pub validity: u8,
    /// The timing bucket the pass carries.
    pub timing: TimingCategory,
    /// The timing bucket the stage key gives.
    pub key_timing: TimingCategory,
}

/// Every atmosphere stage pass in `graph`, including a secondary view's
/// aerial volume, which reads the aerial stage's charts.
pub fn atmosphere_stages(graph: &CompiledGraph) -> Vec<StageAudit> {
    graph
        .pass_bindings()
        .filter_map(|(_, pass)| {
            let stage = match pass.key? {
                PassKey::Atmosphere(stage) => stage,
                PassKey::Secondary {
                    stage: SecondaryStage::Aerial,
                    ..
                } => AtmosphereStage::Aerial,
                _ => return None,
            };
            let graph_reads = pass
                .reads
                .iter()
                .filter_map(|access| match graph.resource_scope(&access.resource)? {
                    ResourceScope::Persistent { owner, .. } => {
                        crate::atmosphere::graph::chart_of_owner(owner)
                    }
                    _ => None,
                })
                .map(|chart| format!("{chart:?}"))
                .collect();
            Some(StageAudit {
                name: pass.name.to_string(),
                graph_reads,
                table_reads: stage_table_reads(stage),
                role: pass.pipeline.map(|pipeline| pipeline.role),
                key_role: pass
                    .key
                    .and_then(PassKey::pipeline_role)
                    .map_or(u32::MAX, crate::pass_key::PipelineRole::value),
                validity: stage.validity(),
                timing: pass.timing,
                key_timing: pass.key.map_or(TimingCategory::Transfer, PassKey::timing),
            })
        })
        .collect()
}

/// The timestamp slots a frame host gives a slot on `target`, read from the
/// graphs compiled with the target.
pub fn target_timed_pass_count(target: &crate::synthetic::OffscreenTarget) -> u32 {
    target.timed_pass_count()
}

/// The scene timing summary the renderer builds from collected passes.
pub fn scene_timings(passes: &[crate::timing::PassTiming]) -> crate::timing::SceneGpuTimings {
    crate::timing::SceneGpuTimings::new(passes)
}

/// Feed a raw timestamp pair through the collector's validation.
#[cfg(feature = "test-support")]
#[must_use]
pub fn counter_refusal(
    start: u64,
    end: u64,
    last_counter: u64,
) -> Option<crate::timing::TimingRefusal> {
    match crate::timing::validate_pair(start, end, last_counter) {
        Err(crate::timing::TimingError::InvalidTimestamp { reason, .. }) => Some(reason),
        _ => None,
    }
}

/// Successful graph compiles on this test thread, split by surface ownership.
#[cfg(feature = "test-support")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SurfaceGraphCompiles {
    /// Host scene-only graphs.
    pub scene_only: usize,
    /// Host scene graphs that include a surface consumer.
    pub scene_and_surface: usize,
    /// Standalone surface-only graphs.
    pub surface_only: usize,
}

#[cfg(feature = "test-support")]
thread_local! {
    static COMPILES: std::cell::Cell<SurfaceGraphCompiles> = const {
        std::cell::Cell::new(SurfaceGraphCompiles {
            scene_only: 0,
            scene_and_surface: 0,
            surface_only: 0,
        })
    };
}

/// Successful graph compiles since this test thread started.
#[cfg(feature = "test-support")]
#[must_use]
pub fn surface_graph_compiles() -> SurfaceGraphCompiles {
    COMPILES.get()
}

#[cfg(feature = "test-support")]
pub(crate) fn compiled(pass_count: usize, has_surface: bool) {
    let mut counts = COMPILES.get();
    if !has_surface {
        counts.scene_only += 1;
    } else if pass_count == 1 {
        counts.surface_only += 1;
    } else {
        counts.scene_and_surface += 1;
    }
    COMPILES.set(counts);
}
