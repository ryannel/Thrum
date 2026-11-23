//! Public renderer failures support Display and source traversal.
use std::error::Error;

use thrum_render::{
    abi::AbiError,
    capabilities::{HardwareLaunchError, RecipeError, SurfaceConfigError},
    contribution::{HookPointError, QualityError, SecondaryViewError},
    error::{ArenaError, EffectGraphError, FrameGpuError, RenderError},
    graph::GraphError,
    present::PresentError,
    runtime::{FrameError, SurfaceFault},
    synthetic::mesh_rows::MeshRowError,
    timing::TimingError,
};

#[test]
fn all_public_errors_implement_error() {
    fn assert_error<E: Error + 'static>() {}

    assert_error::<AbiError>();
    assert_error::<ArenaError>();
    assert_error::<EffectGraphError>();
    assert_error::<FrameError>();
    assert_error::<FrameGpuError>();
    assert_error::<GraphError>();
    assert_error::<HardwareLaunchError>();
    assert_error::<HookPointError>();
    assert_error::<MeshRowError>();
    assert_error::<PresentError>();
    assert_error::<QualityError>();
    assert_error::<RecipeError>();
    assert_error::<SecondaryViewError>();
    assert_error::<SurfaceConfigError>();
    assert_error::<SurfaceFault>();
    assert_error::<RenderError>();
    assert_error::<TimingError>();

    #[cfg(not(target_arch = "wasm32"))]
    {
        struct Request;
        assert_error::<thrum_render::frame::SendFault<Request>>();
    }
}

#[test]
fn presentation_failure_preserves_the_graph_cause() {
    let frame = FrameError::gpu(
        "prepare surface",
        PresentError::Graph(GraphError::DependencyCycle),
    );
    assert!(matches!(
        frame,
        FrameError::Gpu {
            error: FrameGpuError::Present(PresentError::Graph(GraphError::DependencyCycle)),
            ..
        }
    ));
    assert_eq!(
        frame.to_string(),
        "frame GPU: prepare surface: Presentation graph: Live pass dependencies form a cycle"
    );
    let gpu = frame.source().expect("GPU operation cause");
    let present = gpu.source().expect("presenter cause");
    let graph = present.source().expect("graph cause");
    assert!(matches!(
        graph.downcast_ref::<GraphError>(),
        Some(GraphError::DependencyCycle)
    ));
}
