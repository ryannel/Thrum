//! Scene variants for the pass-registry tests: every lighting, shadow,
//! culling, local-light, reconstruction, recipe, capture source and readback
//! choice the 3D scene graph branches on, each compiled on Metal.

use thrum_render::{
    atmosphere::AtmosphereFrame,
    graph::CompiledGraph,
    synthetic::{
        CaptureSource, CullMode, DisplayOverlay, FrameLighting, OffscreenTarget, PreparedSynthetic,
        Reconstruction, SyntheticLight, SyntheticPointLight, SyntheticScene, SyntheticView,
    },
};

/// One compiled graph and the prepared scene it came from.
pub(crate) struct Variant {
    /// What the variant selects, for failure messages.
    pub(crate) label: String,
    /// The compiled graph.
    pub(crate) graph: CompiledGraph,
    /// The prepared scene's timestamp slot count.
    pub(crate) scene_pass_count: u32,
    /// The slot count a frame host reads from this variant's target.
    pub(crate) target_pass_count: u32,
}

#[derive(Clone, Copy)]
enum Light {
    Diagnostic,
    Day,
    Stars,
}

fn scene(
    light: Light,
    shadows: bool,
    culling: CullMode,
    lamps: bool,
    sampling: Reconstruction,
    overlay: DisplayOverlay,
) -> (SyntheticScene, FrameLighting) {
    let mut scene = SyntheticScene::grid(4).expect("grid fixture");
    scene.sun_shadows = shadows;
    scene.camera_culling = culling;
    if lamps {
        scene.local_lights = vec![SyntheticPointLight {
            position: [0.0, 1.0, 0.0],
            radius: 1.0,
            candela: [100.0; 3],
        }];
    }
    if matches!(light, Light::Stars) {
        scene.star_catalogue = Some(&[[0.0, 1.0, 0.0, 0.0001]]);
    }
    scene.view = SyntheticView::perspective(
        [0.0, 2.0, 5.0],
        [0.0; 3],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        100.0,
    )
    .expect("perspective view")
    .with_celestial_basis([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    .expect("celestial basis")
    .with_reconstruction(sampling == Reconstruction::Taau)
    .with_reconstruction_reference(sampling == Reconstruction::Reference)
    .with_temporal_inputs(overlay == DisplayOverlay::TemporalInputs)
    .with_display_overlay(overlay);
    let lighting = match light {
        Light::Diagnostic => FrameLighting::Diagnostic(SyntheticLight::default()),
        Light::Day | Light::Stars => FrameLighting::Atmosphere(
            AtmosphereFrame::new(
                [0.2, 0.8, 0.1],
                if matches!(light, Light::Stars) {
                    0.0
                } else {
                    100_000.0
                },
                [1.0; 3],
                2.0,
                1,
            )
            .expect("atmosphere frame")
            .with_nightglow(),
        ),
    };
    (scene, lighting)
}

/// Every variant, or `None` when no Metal device is available.
pub(crate) fn variants(test: &str) -> Option<Vec<Variant>> {
    let gpu = thrum_test_support::gpu(test, Some(wgpu::Backend::Metal))?;
    let mut variants = Vec::new();
    for recipe in ["portable", "apple-tile"] {
        let (device, queue, caps) = gpu
            .device_with_hardware(
                thrum_test_support::Profile::Portable,
                &thrum_test_support::Restriction::default(),
                11,
                &[],
            )
            .expect("Metal test device");
        let caps = caps
            .with_recipe_override(recipe)
            .expect("Metal supports the recipe");
        let mut arms = Vec::new();
        for light in [Light::Diagnostic, Light::Day, Light::Stars] {
            for shadows in [false, true] {
                for culling in [CullMode::Direct, CullMode::Frustum, CullMode::Occlusion] {
                    for lamps in [false, true] {
                        arms.push((
                            light,
                            shadows,
                            culling,
                            lamps,
                            Reconstruction::Off,
                            DisplayOverlay::Off,
                        ));
                    }
                }
            }
        }
        for (sampling, overlay) in [
            (Reconstruction::Taau, DisplayOverlay::Off),
            (Reconstruction::Reference, DisplayOverlay::Off),
            (Reconstruction::Off, DisplayOverlay::TemporalInputs),
        ] {
            for light in [Light::Day, Light::Stars] {
                arms.push((light, true, CullMode::Frustum, true, sampling, overlay));
            }
        }
        for (light, shadows, culling, lamps, sampling, overlay) in arms {
            let (scene, lighting) = scene(light, shadows, culling, lamps, sampling, overlay);
            let prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
                &caps, &queue, &scene, lighting, 1.0,
            ))
            .expect("prepare the scene");
            let scene_pass_count = prepared.scene_pass_count();
            for source in [CaptureSource::Sdr, CaptureSource::SceneLinear] {
                // Building the target binds every pass's recorder from its key;
                // a pass without one fails here.
                let target =
                    OffscreenTarget::with_capture_source(&device, &prepared, 16, 16, source)
                        .expect("bind the scene graph");
                let target_pass_count =
                    thrum_render::registry_audit::target_timed_pass_count(&target);
                for capture in [false, true] {
                    let label = format!(
                        "recipe={recipe} light={} shadows={shadows} culling={culling:?} \
                         lamps={lamps} sampling={sampling:?} overlay={overlay:?} \
                         source={source:?} capture={capture}",
                        match light {
                            Light::Diagnostic => "diagnostic",
                            Light::Day => "day",
                            Light::Stars => "stars",
                        }
                    );
                    let graph = prepared
                        .graph(&target, capture)
                        .unwrap_or_else(|error| panic!("{label}: {error:?}"));
                    variants.push(Variant {
                        label,
                        graph,
                        scene_pass_count,
                        target_pass_count,
                    });
                }
            }
        }
    }
    Some(variants)
}
