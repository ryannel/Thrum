//! Metal graph, resource, admission and unchanged 3D closure witnesses.

#[cfg_attr(
    not(feature = "creation-counters"),
    expect(
        dead_code,
        reason = "only the counted generation inventory opens a later fixture generation"
    )
)]
mod support {
    pub(crate) mod sprite_fixture;
    pub(crate) mod sprite_graph_snapshot;
}
use support::{sprite_fixture as fixture, sprite_graph_snapshot as graph_snapshot};

use std::time::Duration;
use thrum_render::{
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectCadence,
        EffectGraph, EffectPass, SceneContribution, SceneDrawStage, SceneSurface,
        SpriteContributionSurface,
    },
    runtime::{FrameError, FrameHost, SceneKind, SpriteSceneRequests, SurfaceOutput},
    sprite::{SpriteRect, SpriteRow, SpriteScene},
    synthetic::{CaptureSource, CullMode, Reconstruction},
};

const EXTENT: [u32; 2] = [64, 48];
const PLAIN: [&str; 3] = [
    "sprites",
    "display-referred tone encode",
    "V1 surface present",
];
const CAPTURE: [&str; 4] = [
    "sprites",
    "display-referred tone encode",
    "output readback",
    "V1 surface present",
];

fn inputs() -> (fixture::Scene, SpriteScene) {
    (
        fixture::Scene::new(vec![SpriteRow::new(0, [0.0, 0.0])], 2),
        SpriteScene::new(
            fixture::sheet(
                [1, 1],
                &[225, 6, 104, 255],
                &[SpriteRect {
                    origin: [0; 2],
                    size: [1; 2],
                }],
            ),
            32,
        ),
    )
}

#[test]
#[cfg(feature = "creation-counters")]
fn generation_prepares_only_the_sprite_and_output_resources() {
    for recipe in fixture::RECIPES {
        let Some(thrum_test_support::Opened {
            gpu,
            device,
            queue,
            capabilities: caps,
        }) = fixture::open("2D generation inventory", recipe)
        else {
            return;
        };
        assert!(caps.features().contains(wgpu::Features::TIMESTAMP_QUERY));
        let (policy, scene) = inputs();
        settle_uploads(&device, &queue);
        let before = allocations(&device);
        let mut host = pollster::block_on(FrameHost::new_2d(
            &caps,
            &queue,
            fixture::scene_config(EXTENT),
            fixture::World::default(),
            policy,
            scene,
            SpriteSceneRequests::default(),
        ))
        .expect("prepare the sprite scene");
        // Retire the one-off staging for the sheet and frame table before
        // inspecting the live generation, with no scene frame submitted yet.
        settle_uploads(&device, &queue);
        let counts = allocations(&device);
        // Cross-check the named inventory against every live HAL buffer/texture/query.
        // The sheet, shared transparent placeholder, and shared frame, row and
        // order buffers. Two slots each bind colour, SDR and a view, plus the
        // host's timestamp query, resolve and readback.
        assert_eq!(
            (
                counts.0 - before.0,
                counts.1 - before.1,
                counts.2 - before.2
            ),
            (9, 6, 2)
        );
        assert_eq!(host.scene_kind(), SceneKind::TwoD);
        assert_eq!(host.resolved_reconstructor(), None);
        assert_eq!(
            host.scene_graph()
                .expect("inspect the prepared graph")
                .pass_names()
                .collect::<Vec<_>>(),
            PLAIN
        );
        assert_eq!(
            host.scene_graph_variant(true)
                .expect("inspect the prepared graph")
                .pass_names()
                .collect::<Vec<_>>(),
            CAPTURE
        );
        let storage = host.storage_report();
        let slot = "sprite colour Rgba16Float bytes=24576\nSDR output Rgba8Unorm bytes=12288\n\
            sprite view bytes=32\nsprite rows bytes=3584\nsprite order bytes=128\nunique textures/nominal bytes=(2, 36864)";
        assert_eq!(
            storage,
            format!(
                "embedded sprite sheet Rgba16Float 12x12\nsprite frames bytes=32\nslot=0\n{slot}\nslot=1\n{slot}"
            )
        );
        let pipelines = host.creation_report().pipelines;
        assert_eq!(pipelines.iter().map(|row| row.render).sum::<u64>(), 3);
        assert_eq!(
            pipelines
                .iter()
                .map(|row| row.compute + row.mesh)
                .sum::<u64>(),
            0
        );
        let output = fixture::drawable(&device, EXTENT);
        let report = host
            .drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &output,
                    extent: EXTENT,
                }),
            )
            .expect("drive the real frame executor");
        assert!(report.surface_presented);
        let completed = fixture::drain(&mut host);
        assert_eq!(completed.len(), 1);
        let timings = completed[0]
            .gpu_pass_ns
            .as_ref()
            .expect("required prepared state or timing sample");
        assert_eq!(
            timings
                .passes()
                .iter()
                .map(|pass| pass.name.as_ref())
                .collect::<Vec<_>>(),
            PLAIN
        );

        let (device3, queue3, caps3) = fixture::metal(&gpu, recipe, 2);
        settle_uploads(&device3, &queue3);
        let before3 = allocations(&device3);
        let (scene, light) = graph_snapshot::scene(
            true,
            CullMode::Frustum,
            Reconstruction::Taau,
            thrum_render::synthetic::DisplayOverlay::Off,
        );
        let mut scene = scene;
        scene
            .local_lights
            .push(thrum_render::synthetic::SyntheticPointLight {
                position: [0.0, 1.0, 3.0],
                radius: 4.0,
                candela: [100.0; 3],
            });
        let extent = [64, 48];
        let control_config = thrum_render::runtime::FrameConfig {
            render_extent: extent,
            surface: Some(fixture::profile(extent)),
        };
        let mut policy = fixture::Scene::new(vec![], 1);
        policy.view = scene.view;
        policy.lighting = Some(light);
        let mut control = pollster::block_on(FrameHost::new(
            &caps3,
            &queue3,
            control_config,
            fixture::World::default(),
            policy,
            scene,
            1.0,
        ))
        .expect("prepare the 3D control");
        let counts3 = allocations(&device3);
        assert!(counts3.0 - before3.0 > counts.0 - before.0);
        assert!(counts3.1 - before3.1 > counts.1 - before.1);
        assert_eq!(control.scene_kind(), SceneKind::ThreeD);
        assert_eq!(
            control.resolved_reconstructor(),
            Some(thrum_render::capabilities::Reconstructor::Taau)
        );
        assert!(
            control
                .pipeline_creation_count()
                .expect("creation instrumentation is enabled")
                > 3
        );
        for role in ["depth", "motion", "reactive"] {
            assert!(control.storage_report().contains(&format!("role={role}")));
        }
        let graph = control.scene_graph().expect("inspect the prepared graph");
        let names = graph.pass_names().collect::<Vec<_>>();
        for pass in [
            "atmosphere sky view",
            "atmosphere stars",
            "synthetic Sun shadow near",
            "synthetic cluster build",
            "camera visibility",
            "synthetic forward opaque",
            "portable TAAU",
        ] {
            assert!(
                names.contains(&pass),
                "3D control missing {pass}: {names:?}"
            );
        }
        let output3 = fixture::drawable(&device3, extent);
        assert!(
            control
                .drive(
                    Duration::ZERO,
                    Some(&SurfaceOutput {
                        view: &output3,
                        extent
                    })
                )
                .expect("drive the real frame executor")
                .surface_presented
        );
        let completed = fixture::drain(&mut control);
        assert!(
            completed[0]
                .gpu_pass_ns
                .as_ref()
                .expect("required prepared state or timing sample")
                .get("synthetic forward opaque")
                .is_some()
        );
        println!(
            "{recipe}: 2D/3D HAL buffers, textures, queries {counts:?}/{counts3:?}; 3D named passes {names:?}"
        );
    }
}

#[cfg(feature = "creation-counters")]
fn allocations(device: &wgpu::Device) -> (isize, isize, isize) {
    let hal = device.get_internal_counters().hal;
    (
        hal.buffers.read(),
        hal.textures.read(),
        hal.query_sets.read(),
    )
}

#[cfg(feature = "creation-counters")]
fn settle_uploads(device: &wgpu::Device, queue: &wgpu::Queue) {
    let uploads = queue.submit([]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(uploads),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("retire pending initialization uploads");
}

#[test]
fn rejects_each_3d_request_without_changing_the_sprite_scene() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("2D admission", fixture::RECIPES[0])
    else {
        return;
    };
    let (source, sprites) = inputs();
    let mut host = fixture::host(
        &caps,
        &queue,
        EXTENT,
        source.clone(),
        sprites.sheets[0].sheet.clone(),
    );
    assert!(host.quality_knobs().is_empty());
    assert!(matches!(
        host.set_quality_knob("Sun penumbra", 1),
        Err(thrum_render::contribution::QualityError::NoContribution)
    ));
    let reference = fixture::capture(&mut host, &device, EXTENT, CaptureSource::Sdr).rgba;
    for (label, requests) in [
        (
            "temporal sampling",
            SpriteSceneRequests {
                temporal_sampling: true,
                ..Default::default()
            },
        ),
        (
            "camera culling mode Direct",
            SpriteSceneRequests {
                culling: Some(CullMode::Direct),
                ..Default::default()
            },
        ),
        (
            "camera culling mode Frustum",
            SpriteSceneRequests {
                culling: Some(CullMode::Frustum),
                ..Default::default()
            },
        ),
        (
            "camera culling mode Occlusion",
            SpriteSceneRequests {
                culling: Some(CullMode::Occlusion),
                ..Default::default()
            },
        ),
        (
            "reconstructor Taau",
            SpriteSceneRequests {
                reconstruction: Reconstruction::Taau,
                ..Default::default()
            },
        ),
        (
            "reconstructor Reference",
            SpriteSceneRequests {
                reconstruction: Reconstruction::Reference,
                ..Default::default()
            },
        ),
    ] {
        let result = pollster::block_on(FrameHost::new_2d(
            &caps,
            &queue,
            fixture::scene_config(EXTENT),
            fixture::World::default(),
            source.clone(),
            sprites.clone(),
            requests,
        ));
        let error = result.err().expect("3D-only request was admitted");
        assert!(error.to_string().contains(label), "{error}");
        assert_unchanged(&mut host, &device, &reference);
    }
    for (stage, late, label) in [
        (Some(SceneDrawStage::Shadow), false, "shadow stage"),
        (Some(SceneDrawStage::Opaque), false, "opaque stage"),
        (None, true, "after-opaque stage"),
    ] {
        let entry = Contribution {
            stage,
            late,
            ..Default::default()
        };
        let error = host.set_scene_contribution(Box::new(entry)).unwrap_err();
        assert!(error.to_string().contains(label), "{error}");
        assert_unchanged(&mut host, &device, &reference);
    }
}

fn assert_unchanged(host: &mut fixture::Host, device: &wgpu::Device, reference: &[u8]) {
    assert_eq!(host.scene_kind(), SceneKind::TwoD);
    assert_eq!(
        host.scene_graph()
            .expect("inspect the prepared graph")
            .pass_names()
            .collect::<Vec<_>>(),
        PLAIN
    );
    assert_eq!(
        host.scene_graph_variant(true)
            .expect("inspect the prepared capture graph")
            .pass_names()
            .collect::<Vec<_>>(),
        CAPTURE
    );
    assert_eq!(
        fixture::capture(host, device, EXTENT, CaptureSource::Sdr).rgba,
        reference
    );
}

#[test]
fn attaches_and_runs_only_upload_and_simulate_before_sprites() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("2D upload and simulate", "portable")
    else {
        return;
    };
    let (mut source, sprites) = inputs();
    source.centre = [10.0, 5.0];
    source.rows[0].position = [13.0, 3.0];
    let mut host = fixture::host(
        &caps,
        &queue,
        EXTENT,
        source,
        sprites.sheets[0].sheet.clone(),
    );
    let entry = Contribution::default();
    let actions = std::sync::Arc::clone(&entry.actions);
    host.set_scene_contribution(Box::new(entry))
        .expect("attach the upload and simulation contribution");
    assert_eq!(
        host.scene_graph()
            .expect("inspect the prepared graph")
            .pass_names()
            .collect::<Vec<_>>(),
        [
            "contribution upload",
            "effect 0: simulation witness",
            PLAIN[0],
            PLAIN[1],
            PLAIN[2]
        ]
    );
    let output = fixture::drawable(&device, EXTENT);
    assert!(
        host.drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &output,
                extent: EXTENT
            })
        )
        .expect("drive the real frame executor")
        .surface_presented
    );
    let completed = fixture::drain(&mut host);
    assert_eq!(
        *actions.lock().expect("contribution log is not poisoned"),
        ["rebuild", "upload", "simulate", "submitted"]
    );
    let times = completed[0]
        .gpu_pass_ns
        .as_ref()
        .expect("required prepared state or timing sample");
    assert!(times.get("effect 0: simulation witness").is_some());
    let image = fixture::capture(&mut host, &device, EXTENT, CaptureSource::Sdr);
    let pixel = (28 * EXTENT[0] as usize + 38) * 4;
    assert_eq!(image.rgba[pixel..pixel + 4], [225, 6, 104, 255]);
    assert_eq!(image.rgba[pixel - 4..pixel], [0, 0, 0, 255]);
    println!(
        "Metal upload and simulate see camera (10, 5) and project the sprite pivot to (38, 28)"
    );
}

#[derive(Default)]
struct Contribution {
    stage: Option<SceneDrawStage>,
    late: bool,
    prepared: Option<(wgpu::Buffer, wgpu::ComputePipeline)>,
    actions: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
}

fn assert_contribution_view(frame: &ContributionFrame) {
    assert_eq!(
        frame.camera.map(f32::to_bits),
        [10.0_f32, 5.0, 0.0].map(f32::to_bits)
    );
    let clip = glam::Mat4::from_cols_array_2d(&frame.view_proj)
        .project_point3(glam::Vec3::new(13.0, 3.0, 0.0));
    let x = (clip.x + 1.0) * 32.0;
    let y = (1.0 - clip.y) * 24.0;
    assert!(
        (x - 38.0).abs() < 1e-5 && (y - 28.0).abs() < 1e-5,
        "{x}, {y}"
    );
}

impl SceneContribution for Contribution {
    fn draw_stages(&self) -> Vec<SceneDrawStage> {
        self.stage.into_iter().collect()
    }
    fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), FrameError> {
        panic!("called 3D rebuild")
    }
    fn rebuild_2d(&mut self, surface: &SpriteContributionSurface<'_>) -> Result<(), FrameError> {
        self.actions
            .lock()
            .expect("contribution log is not poisoned")
            .push("rebuild");
        let device = surface.capabilities.site();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite contribution staging witness"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl("@compute @workgroup_size(1) fn update() {}".into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("update"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        self.prepared = Some((buffer, pipeline));
        Ok(())
    }
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            passes: vec![EffectPass {
                name: "simulation witness".into(),
                cadence: EffectCadence::EveryFrame,
                async_eligible: false,
                accesses: vec![],
            }],
            after_opaque: self
                .late
                .then(|| AfterOpaquePass {
                    name: "late draw".into(),
                    kind: AfterOpaqueKind::Render,
                    cadence: EffectCadence::EveryFrame,
                    accesses: vec![],
                    depth: false,
                    write_depth: false,
                    refraction: false,
                })
                .into_iter()
                .collect(),
            ..EffectGraph::default()
        }
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        assert_contribution_view(frame);
        self.actions
            .lock()
            .expect("contribution log is not poisoned")
            .push("upload");
        encoder.clear_buffer(
            &self
                .prepared
                .as_ref()
                .expect("required prepared state or timing sample")
                .0,
            0,
            None,
        );
    }
    fn encode_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        assert_contribution_view(frame);
        assert_eq!(index, 0);
        self.actions
            .lock()
            .expect("contribution log is not poisoned")
            .push("simulate");
        pass.set_pipeline(
            &self
                .prepared
                .as_ref()
                .expect("required prepared state or timing sample")
                .1,
        );
        pass.dispatch_workgroups(1, 1, 1);
    }
    fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {
        panic!("unexpected legacy compute")
    }
    fn draw(&self, _: &mut wgpu::RenderPass<'_>) {
        panic!("unexpected opaque draw")
    }
    fn submitted(&mut self, advanced: bool) {
        assert!(advanced);
        self.actions
            .lock()
            .expect("contribution log is not poisoned")
            .push("submitted");
    }
}

#[test]
fn retained_3d_graphs_match_the_starting_commit() {
    let Some(gpu) = thrum_test_support::gpu("3D graph baseline", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let actual = graph_snapshot::snapshot(&gpu);
    let baseline = include_str!("support/scene-3d-starting.txt");
    assert_eq!(
        actual, baseline,
        "3D pass order, attachment residency or transient lifetimes changed"
    );
}
