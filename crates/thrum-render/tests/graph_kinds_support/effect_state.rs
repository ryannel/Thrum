//! EFFECT-GRAPH M2 submission clock and independent-history contract.
use std::time::Duration;
use thrum_render::contribution::{EffectState, EffectStep, EffectTime, HistoryBinding as H};
use thrum_render::error::EffectGraphError;

#[test]
fn late_depth_writes_refuse_sampling_and_compute_but_allow_a_surface() {
    use thrum_render::contribution::{
        AfterOpaqueKind, AfterOpaquePass, EffectCadence, EffectGraph,
    };
    let mut graph = EffectGraph {
        after_opaque: vec![AfterOpaquePass {
            name: "displaced surface".into(),
            kind: AfterOpaqueKind::Render,
            cadence: EffectCadence::EveryFrame,
            accesses: vec![],
            depth: false,
            write_depth: true,
            refraction: false,
            images: thrum_render::contribution::AfterOpaqueImages::None,
        }],
        ..EffectGraph::default()
    };
    graph.validate().expect("a surface may attach depth");
    graph.after_opaque[0].depth = true;
    assert!(matches!(
        graph.validate(),
        Err(EffectGraphError::AfterOpaqueDepthConflict(_))
    ));
    graph.after_opaque[0].depth = false;
    graph.after_opaque[0].kind = AfterOpaqueKind::Compute;
    assert!(matches!(
        graph.validate(),
        Err(EffectGraphError::AfterOpaqueDepthConflict(_))
    ));
}

fn time(tick: u64) -> EffectTime {
    EffectTime {
        epoch: 7,
        timeline: 3,
        tick,
        tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        alpha: 0.5,
        display_phase: 0.5,
        capture: false,
    }
}

#[test]
fn state_table_and_independent_history_submission() {
    let mut state = EffectState::new(2);
    state.rebuild(1, 1);
    assert_eq!(state.begin(time(10), 8, [12, 14]), EffectStep::Reset);
    state.produced(0);
    state.produced(1);
    assert_eq!(state.side(0, H::Result), 1);
    state.submitted();
    assert_eq!(state.anchor(), [12, 14]);
    for _ in 0..20 {
        assert_eq!(state.begin(time(10), 8, [12, 14]), EffectStep::Keep);
        assert_eq!(state.side(0, H::Result), 1);
        state.submitted();
    }
    assert_eq!(
        state.begin(time(11), 8, [16, 14]),
        EffectStep::Advance {
            ticks: 1,
            dropped: 0
        }
    );
    assert_eq!(state.anchor_shift(), [4, 0]);
    state.produced(0);
    assert_eq!(state.side(0, H::Result), 0);
    assert_eq!(state.side(1, H::Result), 1);
    // Discard: no submission callback, and the next recording resolves from committed B.
    assert_eq!(
        state.begin(time(11), 8, [12, 14]),
        EffectStep::Advance {
            ticks: 1,
            dropped: 0
        }
    );
    assert_eq!(state.anchor(), [12, 14]);
    assert_eq!(state.side(0, H::Previous), 1);
    state.produced(0);
    state.submitted();
    assert_eq!(state.side(0, H::Previous), 0);
    assert_eq!(state.side(1, H::Previous), 1);
    let mut capture = time(1000);
    capture.capture = true;
    capture.timeline = 9;
    assert_eq!(state.begin(capture, 8, [999, 999]), EffectStep::Keep);
    state.produced(1);
    state.submitted();
    assert_eq!(state.committed().expect("submitted").tick, 11);
    assert_eq!(
        state.begin(time(100), 8, [12, 14]),
        EffectStep::Advance {
            ticks: 8,
            dropped: 81
        }
    );
    state.submitted();
    assert_eq!(state.dropped_ticks(), 81);
    assert_eq!(state.begin(time(99), 8, [0, 0]), EffectStep::Reset);
    state.submitted();
    let mut branch = time(99);
    branch.timeline += 1;
    assert_eq!(state.begin(branch, 8, [0, 0]), EffectStep::Reset);
    state.submitted();
    state.rebuild(1, 1);
    assert_eq!(state.begin(branch, 8, [0, 0]), EffectStep::Keep);
    state.rebuild(2, 1);
    assert_eq!(state.begin(branch, 8, [0, 0]), EffectStep::Reset);
    // A discarded full frame must be full again.
    assert_eq!(state.begin(branch, 8, [0, 0]), EffectStep::Reset);
}

use thrum_render::{
    contribution::{EffectAccess, EffectCadence as C, EffectGraph, EffectPass},
    graph::{AccessKind as A, ResourceDescriptor as D, ResourceScope as S},
};
fn declaration(scope: S, cadence: C) -> EffectGraph {
    EffectGraph {
        tick_passes: Vec::new(),
        view_volume_media: Vec::new(),
        sun_paths: Vec::new(),
        opaque_reads: Vec::new(),
        after_opaque: Vec::new(),
        resources: vec![(
            D::Buffer {
                bytes: 4,
                usage: wgpu::BufferUsages::STORAGE,
            },
            scope,
        )],
        passes: vec![EffectPass {
            name: "producer".into(),
            cadence,
            async_eligible: false,
            accesses: vec![EffectAccess {
                resource: 0,
                range: 0..4,
                kind: A::StorageWrite,
                view: None,
                history: matches!(scope, S::History { .. }).then_some(H::Next),
            }],
        }],
    }
}
#[test]
fn cadence_refuses_transients_imports_and_loaded_attachments() {
    for cadence in [C::Skippable, C::TimeSliced] {
        for scope in [S::Transient, S::Imported { initialized: true }] {
            assert!(matches!(
                declaration(scope, cadence).validate(),
                Err(EffectGraphError::CadenceRequiresRetainedOutput(_))
            ));
        }
        let mut graph = declaration(
            S::Persistent {
                owner: 1,
                epoch: 1,
                initialized: true,
            },
            cadence,
        );
        graph.passes[0].accesses[0].kind = A::AttachmentLoad;
        assert!(matches!(
            graph.validate(),
            Err(EffectGraphError::CadenceRequiresRetainedOutput(_))
        ));
    }
}
#[test]
fn history_refuses_partial_updates_duplicate_producers_and_early_result() {
    let scope = S::History {
        owner: 1,
        epoch: 1,
        initialized: true,
    };
    assert!(matches!(
        declaration(scope, C::TimeSliced).validate(),
        Err(EffectGraphError::TimeSlicedHistory(_))
    ));
    for cadence in [C::Skippable, C::EveryFrame] {
        let mut graph = declaration(scope, C::Skippable);
        let mut other = graph.passes[0].clone();
        other.name = "other".into();
        other.cadence = cadence;
        graph.passes.push(other);
        assert_eq!(
            graph.validate().expect_err("second producer"),
            EffectGraphError::DuplicateHistoryProducer
        );
    }
    let mut graph = declaration(scope, C::Skippable);
    let mut reader = graph.passes[0].clone();
    reader.name = "too early".into();
    reader.accesses[0].kind = A::StorageRead;
    reader.accesses[0].history = Some(H::Result);
    graph.passes.insert(0, reader);
    assert_eq!(
        graph.validate().expect_err("consumer before producer"),
        EffectGraphError::ResultBeforeProducer
    );
}

#[expect(
    dead_code,
    reason = "the graph kinds test also uses the texture roundtrip fixture"
)]
mod support;

#[test]
fn absent_effect_preserves_day_night_capture_graphs_exactly() {
    let Some(gpu) = thrum_test_support::gpu(
        "absent_effect_preserves_day_night_capture_graphs_exactly",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    support::prove_scenes(&gpu);
}

#[test]
fn a_second_declaration_cannot_hide_a_second_producer_of_the_same_pair() {
    let mut graph = declaration(
        S::History {
            owner: 1,
            epoch: 1,
            initialized: true,
        },
        C::Skippable,
    );
    graph.resources.push(graph.resources[0].clone());
    let mut second = graph.passes[0].clone();
    second.name = "second producer".into();
    second.accesses[0].resource = 1;
    graph.passes.push(second);
    assert_eq!(
        graph
            .validate()
            .expect_err("logical pair identities must be unique"),
        EffectGraphError::DuplicateEffectOwner
    );
}

#[test]
fn async_eligibility_refuses_non_compute_accesses() {
    let mut graph = declaration(
        S::Persistent {
            owner: 1,
            epoch: 1,
            initialized: true,
        },
        C::EveryFrame,
    );
    graph.passes[0].async_eligible = true;
    graph.validate().expect("compute eligible");
    for kind in [
        A::AttachmentLoad,
        A::AttachmentClear,
        A::CopyRead,
        A::CopyWrite,
        A::VertexRead,
        A::IndexRead,
    ] {
        graph.passes[0].accesses[0].kind = kind;
        assert_eq!(
            graph.validate().expect_err("compute only"),
            EffectGraphError::AsyncRequiresCompute("producer".into())
        );
    }
}

#[test]
fn late_passes_share_history_validation_and_refuse_unrecordable_accesses_at_preparation() {
    use thrum_render::contribution::{AfterOpaqueKind as K, AfterOpaquePass};
    let mut graph = declaration(
        S::History {
            owner: 1,
            epoch: 1,
            initialized: true,
        },
        C::EveryFrame,
    );
    let producer = graph.passes.remove(0);
    let late = AfterOpaquePass {
        name: "late producer".into(),
        kind: K::Compute,
        cadence: C::Skippable,
        accesses: producer.accesses,
        depth: true,
        write_depth: false,
        refraction: false,
        images: thrum_render::contribution::AfterOpaqueImages::None,
    };
    graph.after_opaque.push(late.clone());
    graph.validate().expect("late history producer");
    let mut reader = late.clone();
    reader.name = "late result".into();
    reader.kind = K::Render;
    reader.cadence = C::EveryFrame;
    reader.accesses[0].kind = A::StorageRead;
    reader.accesses[0].history = Some(H::Result);
    graph.after_opaque.push(reader.clone());
    graph.validate().expect("late result after producer");
    graph.after_opaque.swap(0, 1);
    assert_eq!(
        graph.validate().expect_err("early result"),
        EffectGraphError::ResultBeforeProducer
    );
    graph.after_opaque = vec![reader];
    for kind in [
        A::AttachmentLoad,
        A::AttachmentClear,
        A::CopyRead,
        A::CopyWrite,
    ] {
        graph.after_opaque[0].accesses[0].kind = kind;
        assert_eq!(
            graph
                .validate()
                .expect_err("unsupported local render access"),
            EffectGraphError::AfterOpaqueUsesSceneAttachments("late result".into())
        );
    }
    graph.after_opaque = vec![late];
    graph.after_opaque[0].cadence = C::TimeSliced;
    assert_eq!(
        graph.validate().expect_err("partial late history"),
        EffectGraphError::TimeSlicedHistory("late producer".into())
    );
}
