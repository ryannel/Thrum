//! EFFECT-GRAPH M2 submission clock and independent-history contract.
use std::time::Duration;
use thrum_render::contribution::{EffectState, EffectStep, EffectTime, HistoryBinding as H};

fn time(tick: u64) -> EffectTime {
    EffectTime {
        epoch: 7,
        timeline: 3,
        tick,
        tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        alpha: 0.5,
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
            assert!(
                declaration(scope, cadence)
                    .validate()
                    .expect_err("skip requires retained output")
                    .starts_with("CadenceRequiresRetainedOutput")
            );
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
        assert!(
            graph
                .validate()
                .expect_err("loaded attachment cannot skip")
                .starts_with("CadenceRequiresRetainedOutput")
        );
    }
}
#[test]
fn history_refuses_partial_updates_duplicate_producers_and_early_result() {
    let scope = S::History {
        owner: 1,
        epoch: 1,
        initialized: true,
    };
    assert!(
        declaration(scope, C::TimeSliced)
            .validate()
            .expect_err("partial history write")
            .starts_with("TimeSlicedHistory")
    );
    for cadence in [C::Skippable, C::EveryFrame] {
        let mut graph = declaration(scope, C::Skippable);
        let mut other = graph.passes[0].clone();
        other.name = "other".into();
        other.cadence = cadence;
        graph.passes.push(other);
        assert_eq!(
            graph.validate().expect_err("second producer"),
            "DuplicateHistoryProducer"
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
        "ResultBeforeProducer"
    );
}

#[cfg(target_os = "macos")]
#[path = "graph_kinds_support/scenes.rs"]
mod scenes;
#[cfg(target_os = "macos")]
fn device(
    generation: u64,
) -> (
    wgpu::Device,
    wgpu::Queue,
    thrum_render::capabilities::DeviceCapabilities,
) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real Metal");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Metal device");
        let caps =
            thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, generation);
        (device, queue, caps)
    })
}
#[test]
#[cfg(target_os = "macos")]
fn absent_effect_preserves_day_night_capture_graphs_exactly() {
    scenes::prove_scenes();
}
