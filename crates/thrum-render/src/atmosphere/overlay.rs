//! Physical sky radiance for the optional world-light producers.

use crate::shaders::{CloudVariant, Shader, wgsl};

const EDGE: u32 = 48;
// Store physical radiance with headroom for the bright sky near the Sun.
// Decode before the BRDF and exposure, just as for the clear-sky encoding.
const GAIN: f32 = 32.0;
pub(crate) const CHART_LIGHTING_BINDING: u32 = 30;
pub(crate) const REFLECTION_TAGS_BINDING: u32 = 31;
pub(crate) const REFLECTION_TAGS_BYTES: u64 = size_of::<[[u32; 4]; 2]>() as u64;

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ChartLighting {
    mode: u32,
    cloudy: u32,
    haze: f32,
    padding: u32,
}

pub(crate) const CHART_LIGHTING_BYTES: u64 = size_of::<ChartLighting>() as u64;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Lighting {
    #[default]
    Clear,
    Overlay,
    OverlayAndHaze,
}

#[derive(Clone)]
pub(super) struct Target {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) lighting: Lighting,
    pub(super) chart_lighting: wgpu::Buffer,
    pub(super) reflection_tags: wgpu::Buffer,
}

impl Target {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue, view_volume: bool) -> Self {
        let size = wgpu::Extent3d {
            width: EDGE,
            height: EDGE,
            depth_or_array_layers: 6,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sky overlay"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // Until the contribution publishes its first valid weather frame the
        // overlay is transparent, including after a device replacement.
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&vec![[0_u16, 0, 0, 0x3c00]; (6 * EDGE * EDGE) as usize]),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(EDGE * 8),
                rows_per_image: Some(EDGE),
            },
            size,
        );
        let overlay = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let chart_lighting = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atmosphere chart lighting"),
            size: CHART_LIGHTING_BYTES,
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let reflection_tags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atmosphere reflection tags"),
            size: REFLECTION_TAGS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        Self {
            texture,
            view: overlay,
            chart_lighting,
            reflection_tags,
            lighting: if view_volume {
                Lighting::Overlay
            } else {
                Lighting::OverlayAndHaze
            },
        }
    }
}

pub(super) fn prepare_diffuse(
    device: &crate::runtime::DeviceSite,
    uniform: &wgpu::Buffer,
    chart: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    environment: &wgpu::Buffer,
    target: &Target,
    (directional, cloud): (&wgpu::Buffer, Option<&super::CloudPrepared>),
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let haze = target.lighting == Lighting::OverlayAndHaze;
    let variant = cloud.map_or(CloudVariant::Absent, super::CloudPrepared::variant);
    let diffuse = pipeline(
        device,
        "atmosphere overlay diffuse irradiance",
        "diffuse_main",
        Shader::AtmosphereOverlayDiffuse(haze, variant),
    );
    let mut entries = vec![
        buffer(0, uniform),
        view(1, chart),
        sampler_entry(2, sampler),
        buffer(3, environment),
        view(5, &target.view),
        buffer(6, directional),
    ];
    let faces = target.texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    entries.push(view(7, &faces));
    if haze && let Some(cloud) = cloud {
        entries.extend([view(13, &cloud.view), buffer(14, &cloud.uniform)]);
        if let Some(band) = &cloud.band {
            entries.extend(band.entries());
        }
    }
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("atmosphere overlay diffuse inputs"),
        layout: &diffuse.get_bind_group_layout(0),
        entries: &entries,
    });
    (diffuse, group)
}

pub(super) fn prepare_chart_lighting(
    device: &crate::runtime::DeviceSite,
    [snapshot, live]: [&wgpu::Buffer; 2],
    chart: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    target: &Target,
    (directional, cloud): (&wgpu::Buffer, Option<&super::CloudPrepared>),
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let haze = target.lighting == Lighting::OverlayAndHaze;
    let variant = cloud.map_or(CloudVariant::Absent, super::CloudPrepared::variant);
    let pipeline = pipeline(
        device,
        "atmosphere chart overlay",
        if haze { "haze_main" } else { "classify_main" },
        Shader::AtmosphereChartOverlay(haze, variant),
    );
    let faces = target.texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let mut entries = vec![view(7, &faces), buffer(8, &target.chart_lighting)];
    if haze {
        entries.extend([
            buffer(0, snapshot),
            view(1, chart),
            sampler_entry(2, sampler),
            buffer(4, live),
            view(5, &target.view),
            buffer(6, directional),
        ]);
        if let Some(cloud) = cloud {
            entries.extend([view(13, &cloud.view), buffer(14, &cloud.uniform)]);
            if let Some(band) = &cloud.band {
                entries.extend(band.entries());
            }
        }
    }
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("atmosphere chart overlay inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    (pipeline, group)
}

pub(crate) fn chart_overlay_source(haze: bool, cloud: CloudVariant) -> String {
    let mut source = format!(
        "{}\n{}\n{}",
        include_str!("chart_lighting.wgsl"),
        include_str!("chart_overlay_classify.wgsl"),
        include_str!("chart_overlay.wgsl"),
    );
    if haze {
        use std::fmt::Write;
        write!(
            source,
            "\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            wgsl!(@lighting_constants),
            wgsl!("atmosphere/storage.wgsl"),
            wgsl!("atmosphere/geometry.wgsl"),
            wgsl!("atmosphere/common.wgsl"),
            wgsl!("directional_light.wgsl"),
            include_str!("chart_overlay_meter.wgsl"),
            include_str!("chart_haze.wgsl"),
            include_str!("overlay.wgsl"),
        )
        .expect("append chart haze shader");
        source.push_str("\n@group(0) @binding(6) var<uniform> overlay_sun: DirectionalLight;\n");
        source.push_str("fn overlay_open_sun() -> f32 { return dot(overlay_sun.irradiance.rgb, REC709_LUMA) * max(overlay_sun.direction_to_light.y, 0.0); }\n");
        append_cloud_factor(&mut source, cloud, "live_atmosphere");
    }
    source
}

fn append_cloud_factor(source: &mut String, cloud: CloudVariant, observer: &str) {
    if cloud.present() {
        use std::fmt::Write;
        source.push_str(&cloud.sampling_source_for(0, observer));
        writeln!(source, "fn overlay_sun_factor() -> f32 {{ return cloud_factor({observer}.camera_altitude.xyz); }}")
            .expect("append cloud factor");
    } else {
        source.push_str("\nfn overlay_sun_factor() -> f32 { return 1.0; }\n");
    }
}

pub(crate) fn chart_consumer_layout(
    visibility: wgpu::ShaderStages,
) -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        crate::binding::uniform(CHART_LIGHTING_BINDING, visibility, CHART_LIGHTING_BYTES),
        crate::binding::uniform(REFLECTION_TAGS_BINDING, visibility, REFLECTION_TAGS_BYTES),
    ]
}

pub(crate) fn chart_consumer_source(source: &str, group: u32) -> String {
    with_reflection_gain(&format!(
        "{source}\n{}\n@group({group}) @binding({CHART_LIGHTING_BINDING}) var<uniform> chart_lighting: ChartLighting;\n@group({group}) @binding({REFLECTION_TAGS_BINDING}) var<uniform> reflection_tags: ReflectionTags;\n{}",
        include_str!("chart_lighting.wgsl"),
        include_str!("chart_reflection_sample.wgsl"),
    ))
}

pub(crate) fn reflection_source() -> String {
    let integrate = wgsl!(@reflection_integrate
        "            let encoded = textureSampleLevel(reflection_sky,reflection_sampler,uv,0.0);\n            if !cloudy {\n                radiance += prepared.w * encoded;\n            } else {\n                radiance += vec4(prepared.w * overlay_radiance(direction,encoded,reflection_sampler) / OVERLAY_REFLECTION_GAIN,0.0);\n            }\n");
    with_reflection_gain(&format!(
        "{}\n{}\n{}\n{}
@group(0) @binding(6) var<uniform> overlay_flags: EnvironmentLight;
@group(0) @binding(7) var<uniform> overlay_chart: ChartLighting;
@group(0) @binding(8) var<storage, read_write> reflection_tags: ReflectionTags;
fn reflection_is_cloudy() -> bool {{
    return select(overlay_chart.cloudy != 0u, overlay_flags.sky_diffuse[2].w != 0.0, overlay_chart.mode == 0u);
}}
fn publish_reflection_encoding(mip: u32, cloudy: bool) {{
    reflection_tags.tags[mip / 4u][mip % 4u] = u32(cloudy);
}}
",
        super::reflection::cube_source_with(integrate),
        include_str!("overlay.wgsl"),
        wgsl!("environment_light.wgsl"),
        include_str!("chart_lighting.wgsl"),
    ))
}

pub(crate) fn diffuse_source(haze: bool, cloud: CloudVariant) -> String {
    let diffuse = wgsl!(@diffuse_producer concat!(
        include_str!("diffuse_nodes.wgsl"),
        include_str!("overlay_diffuse.wgsl"),
        include_str!("overlay.wgsl"),
        include_str!("chart_overlay_classify.wgsl"),
        include_str!("chart_overlay_meter.wgsl"),
    ));
    if !haze {
        return format!(
            "{diffuse}\n{}\n{}\n{}",
            wgsl!(@lighting_constants),
            wgsl!("directional_light.wgsl"),
            include_str!("overlay_diffuse_no_haze.wgsl")
        );
    }
    let mut source = format!(
        "{diffuse}\n{}\n{}\n{}",
        wgsl!(@lighting_constants),
        wgsl!("directional_light.wgsl"),
        include_str!("overlay_haze.wgsl")
    );
    append_cloud_factor(&mut source, cloud, "atmosphere");
    source
}

pub(crate) fn with_reflection_gain(source: &str) -> String {
    format!("const OVERLAY_REFLECTION_GAIN: f32 = {GAIN:?};\n{source}")
}

pub(crate) fn reflection_product(lighting: Lighting) -> &'static str {
    if lighting == Lighting::Clear {
        wgsl!(@reflection_product
            "    let radiance=decode_sky_light(encoded,atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb);\n")
    } else {
        wgsl!(@reflection_product "    let radiance=sample_overlay_reflection(encoded,ray,level,atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb);\n")
    }
}

pub(crate) fn forward_environment(lighting: Lighting, ambient: bool) -> &'static str {
    match (lighting == Lighting::Clear, ambient) {
        (true, false) => wgsl!("synthetic_environment.wgsl"),
        (true, true) => wgsl!(@ambient_environment),
        (false, false) => {
            wgsl!(@environment_surface "    let environment=sample_overlay_reflection(encoded,ray,level,aerial_frame.sun_lux.w*aerial_frame.colour_pre_exposure.rgb);\n")
        }
        (false, true) => {
            wgsl!(@ambient_environment_terms "    let environment=sample_overlay_reflection(encoded,ray,level,aerial_frame.sun_lux.w*aerial_frame.colour_pre_exposure.rgb);\n")
        }
    }
}

pub(crate) fn aerial_source(lighting: Lighting) -> &'static str {
    if lighting == Lighting::OverlayAndHaze {
        wgsl!(@aerial_sample_and_decode "atmosphere/overlay_aerial_decode.wgsl")
    } else {
        wgsl!("synthetic_aerial.wgsl")
    }
}

fn pipeline(
    device: &crate::runtime::DeviceSite,
    name: &str,
    entry: &str,
    shader: Shader,
) -> wgpu::ComputePipeline {
    let module = shader.compile(device, name);
    device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some(name),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn buffer(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn view(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

fn sampler_entry(binding: u32, sampler: &wgpu::Sampler) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::Sampler(sampler),
    }
}
