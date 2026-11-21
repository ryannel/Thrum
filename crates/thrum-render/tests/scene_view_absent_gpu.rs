//! Missing declarations add nothing; idle slots and changed requests allocate
//! nothing in ordinary submitted frames, and their timings report actual work.
mod pre_record_source;
mod support;

// Reverse only REVIEW-RENDER M1's WGSL edits before checking SCENE-VIEWS'
// historical source hashes. The fragments are fixed so later edits fail.
fn before_light_split(source: &str) -> String {
    const RECORDS: &str = "struct DirectionalLight {
    direction_to_light: vec4<f32>,
    irradiance: vec4<f32>,
    half_vector_fresnel: vec4<f32>,
    ambient_irradiance: vec4<f32>,
}

struct EnvironmentLight {
    horizontal_irradiance: vec4<f32>,
    sky_diffuse: array<vec4<f32>,9>,
}

";
    const LEGACY_RECORD: &str = "struct DirectionalSource {
    direction_to_light: vec4<f32>,
    irradiance: vec4<f32>,
    half_vector_fresnel: vec4<f32>,
    ambient_irradiance: vec4<f32>,
    sky_diffuse: array<vec4<f32>,9>,
}
";
    const VIEW: &str = "@group(0) @binding(0) var<uniform> view: ViewRecord;";
    const AMBIENT_SOURCE: &str = "fn ambient_source(n: vec3<f32>) -> vec3<f32> {
    if environment_light.horizontal_irradiance.w==0.0 { return light.ambient_irradiance.rgb; }";
    const LEGACY_AMBIENT: &str = "fn ambient_irradiance(n: vec3<f32>) -> vec3<f32> {
    if light.ambient_irradiance.w==0.0 { return light.ambient_irradiance.rgb; }";
    const SURFACE_WRAPPER: &str = "
fn ambient_irradiance(n: vec3<f32>, world_position: vec3<f32>, pixel_position: vec4<f32>) -> vec3<f32> {
    return ambient_source(n);
}

";
    let mut historical = source.to_owned();
    for (split, original) in [
        (RECORDS, ""),
        (VIEW, &format!("{LEGACY_RECORD}{VIEW}")),
        (
            "@group(1) @binding(0) var<uniform> light: DirectionalLight;\n@group(1) @binding(18) var<uniform> environment_light: EnvironmentLight;",
            "@group(1) @binding(0) var<uniform> light: DirectionalSource;",
        ),
        (AMBIENT_SOURCE, LEGACY_AMBIENT),
        (
            "evaluate_sky_diffuse(environment_light.sky_diffuse,n)",
            "evaluate_sky_diffuse(light.sky_diffuse,n)",
        ),
        (SURFACE_WRAPPER, "\n"),
        (
            "indirect_radiance(surface_base,metalness,roughness,n,v,orm.r,input.world_position,input.position)",
            "indirect_radiance(surface_base,metalness,roughness,n,v,orm.r)",
        ),
        (
            "ao: f32,\n                     world_position: vec3<f32>, pixel_position: vec4<f32>) -> vec3<f32>",
            "ao: f32) -> vec3<f32>",
        ),
        (
            "ambient_irradiance(n,world_position,pixel_position)",
            "ambient_irradiance(n)",
        ),
    ] {
        assert_eq!(
            historical.matches(split).count(),
            1,
            "split WGSL fragment {split:?}"
        );
        historical = historical.replacen(split, original, 1);
    }
    historical
}

fn source_fingerprint(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn pre_view_source(text: &str) -> String {
    // CULLING's independent exact-image fix gives both ordinary
    // fragment entries an explicit full sample mask. Undo only that known
    // upstream patch for this module's historical source oracle; all other
    // bytes must still match the pre-view hashes below.
    const COVERAGE: &str = concat!(
        "// Explicit coverage prevents Metal triangle merging from borrowing a neighbour\n",
        "// primitive's interpolants for derivative/helper lanes. All samples stay covered;\n",
        "// the ordinary depth test still owns visibility. Volumes keep FragmentOutput.\n",
        "struct OpaqueFragmentOutput {\n",
        "    @location(0) colour: vec4<f32>,\n",
        "    @location(1) motion: vec2<f32>,\n",
        "    @location(2) reactive: f32,\n",
        "    @builtin(sample_mask) coverage: u32,\n",
        "}\n",
    );
    if !text.contains("OpaqueFragmentOutput") {
        return text.to_owned();
    }
    assert_eq!(
        text.matches(COVERAGE).count(),
        1,
        "exact upstream coverage declaration"
    );
    let text = text.replacen(COVERAGE, "", 1);
    assert_eq!(text.matches("OpaqueFragmentOutput").count(), 4);
    assert_eq!(text.matches(", 0xffffffffu)").count(), 2);
    text.replace("OpaqueFragmentOutput", "FragmentOutput")
        .replace(", 0xffffffffu)", ")")
}

#[test]
fn every_main_shader_variant_retains_its_pre_view_bytes() {
    use thrum_render::{
        atmosphere::AtmosphereFrame,
        capabilities::DeviceCapabilities,
        synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
    };
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "unchanged main shader",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    // Independently composed from the original scene_source.rs/air_shadow.rs
    // and WGSL files at pre-view main commit 67ed516e2; these are source bytes,
    // not new retained-image comparisons or values generated by the view path.
    for (air, shadow, length, expected) in [
        (false, false, 31_434, 0xcafd_688b_af6c_fa84),
        (false, true, 31_478, 0x8eb8_a33f_c1cb_a60b),
        (true, false, 38_854, 0x7d3e_3a8d_1b43_646a),
        (true, true, 41_489, 0x1f25_ea03_b0f3_b048),
    ] {
        pollster::block_on(async {
            let mut scene = SyntheticScene::grid(1).expect("scene");
            scene.view = support::camera(0.0, 2.0);
            scene.sun_shadows = shadow;
            let lighting = if air {
                FrameLighting::Atmosphere(
                    AtmosphereFrame::new([0.5, 0.707, 0.5], 100_000.0, [1.0; 3], 0.0, 1)
                        .expect("air"),
                )
            } else {
                FrameLighting::Diagnostic(SyntheticLight::default())
            };
            let mut host =
                support::host_with_lighting(&capabilities, &queue, scene, lighting).await;
            let (observer, control) = support::Observer::new(0);
            host.set_scene_contribution(Box::new(observer))
                .expect("no-view observer");
            let control = control.lock().expect("control");
            let m1 = pre_record_source::before_record_composition(&control.main_shader);
            let source = before_light_split(&pre_view_source(&m1));
            assert_eq!(source.len(), length, "air={air} shadow={shadow}");
            assert_eq!(
                source_fingerprint(&source),
                expected,
                "air={air} shadow={shadow}"
            );
        });
    }
}
#[cfg(feature = "creation-counters")]
use std::time::Duration;
#[cfg(feature = "creation-counters")]
use support::Host;
#[cfg(feature = "creation-counters")]
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir},
    runtime::FrameSample,
    synthetic::SyntheticScene,
};

#[cfg(feature = "creation-counters")]
fn frame(host: &mut Host, device: &wgpu::Device, surface: &wgpu::TextureView) -> FrameSample {
    assert!(
        host.advance(Duration::ZERO)
            .expect("advance")
            .encode_scene()
            .expect("ordinary serial frame")
            .submit(Some(&thrum_render::runtime::SurfaceOutput {
                view: surface,
                extent: [64, 64]
            }))
            .expect("submit scene")
            .submitted
    );
    thrum_test_support::wait::until(
        "submitted frame retirement",
        Duration::from_secs(30),
        || {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .expect("frame completion");
            host.poll_completed().expect("completed sample")
        },
    )
}
#[cfg(feature = "creation-counters")]
fn objects(device: &wgpu::Device) -> [isize; 11] {
    let hal = device.get_internal_counters().hal;
    [
        hal.buffers.read(),
        hal.textures.read(),
        hal.texture_views.read(),
        hal.samplers.read(),
        hal.render_pipelines.read(),
        hal.compute_pipelines.read(),
        hal.shader_modules.read(),
        hal.query_sets.read(),
        hal.bind_groups.read(),
        hal.bind_group_layouts.read(),
        hal.pipeline_layouts.read(),
    ]
}

#[cfg(feature = "creation-counters")]
#[test]
fn declarations_and_asks_leave_the_default_shader_and_main_frame_unchanged() {
    use support::*;
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary views absent",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.sun_shadows = true;
        scene.camera_culling = thrum_render::synthetic::CullMode::Frustum;
        scene.meshes = Some(std::sync::Arc::new(
            thrum_render::synthetic::SyntheticMeshes {
                updates: None,
                meshes: Vec::new(),
                slots: vec![thrum_render::synthetic::SyntheticMeshChoice::Cube],
            },
        ));
        scene.view = camera(0.0, 2.0);
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].emissive_cutoff = [1024.0, 512.0, 256.0, 0.0];
        scene.instances[0].current[3] = [0.0, 0.0, 0.5, 1.0];
        scene.instances[0].previous = scene.instances[0].current;
        let mut host = host_with_config(
            &capabilities,
            &queue,
            scene,
            thrum_render::synthetic::FrameLighting::Diagnostic(
                thrum_render::synthetic::SyntheticLight::default(),
            ),
            thrum_render::runtime::FrameConfig {
                render_extent: [64, 64],
                surface: Some(thrum_render::present::SurfaceProfile {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    extent: [64, 64],
                }),
            },
        )
        .await;
        let surface = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("submitted view absence surface"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let base_graph = format!("{:?}", host.scene_graph().expect("base graph"));
        let base_count = host.pipeline_creation_count().expect("creation counters");
        let (empty, empty_control) = Observer::new(0);
        host.set_scene_contribution(Box::new(empty))
            .expect("no declaration");
        assert_eq!(
            base_graph,
            format!("{:?}", host.scene_graph().expect("no-view graph")),
            "passes, resources and pipeline keys remain identical"
        );
        assert_eq!(
            base_count,
            host.pipeline_creation_count().expect("creation counters")
        );
        let base_shader = empty_control
            .lock()
            .expect("empty observer")
            .main_shader
            .clone();
        assert!(
            !base_shader.contains("secondary_view"),
            "no view shader is added to the main source"
        );
        let base = host.capture_pixels().expect("base pixels");
        let (observer, control) = Observer::new(1);
        host.set_scene_contribution(Box::new(observer))
            .expect("slot declaration");
        assert_eq!(base_shader, control.lock().expect("observer").main_shader);
        assert_eq!(base, host.capture_pixels().expect("declared idle pixels"));
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(
                &aimed([0.0, -2.0, 2.0], [0.0, 2.0, -1.5]),
                [64, 64],
                None,
                ViewAir::Camera,
            )
            .expect("ask"),
        );
        assert_eq!(base, host.capture_pixels().expect("asked pixels"));
        // Captures allocate their own detached targets. Use the prepared frame
        // slots below when asserting the absence of runtime object creation.
        control.lock().expect("control").requests[0] = None;
        frame(&mut host, &device, &surface);
        let before = objects(&device);
        assert!(
            before[0] > 0 && before[1] > 0,
            "Metal counters must be live"
        );
        let pipelines = host.pipeline_creation_count().expect("pipeline count");
        for request in [
            None,
            Some(aimed([0.0, -2.0, 2.0], [0.0, 2.0, -1.5])),
            Some(aimed([0.0, -3.0, 2.0], [0.2, 3.0, -1.5])),
            None,
        ] {
            control.lock().expect("control").requests[0] = request.map(|view| {
                SecondaryViewRequest::new(&view, [64, 64], None, ViewAir::Camera).expect("ask")
            });
            let sample = frame(&mut host, &device, &surface);
            let timings = sample.gpu_pass_ns.expect("Metal pass timings");
            let names = timings
                .passes()
                .iter()
                .filter(|pass| pass.name.starts_with("secondary view "))
                .map(|pass| pass.name.to_string())
                .collect::<Vec<_>>();
            assert_eq!(
                names.is_empty(),
                request.is_none(),
                "only asked slots have timings: {names:?}"
            );
            assert_eq!(
                names
                    .iter()
                    .any(|name| name.starts_with("secondary view Sun caster cull")),
                request.is_some(),
                "only an asked view compacts casters: {names:?}"
            );
            assert_eq!(
                before,
                objects(&device),
                "changing or idling the request creates no GPU object"
            );
            assert_eq!(
                pipelines,
                host.pipeline_creation_count().expect("pipeline count")
            );
        }
        assert_eq!(host.creation_report().violations, 0);
    });
}
