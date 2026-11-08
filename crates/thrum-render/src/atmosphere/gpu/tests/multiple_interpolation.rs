//! Same-model GPU diagnostic: change only M's Sun-mu column count. This does
//! not qualify the isotropic closure, transport quadrature or spectral model.
use super::super::{AtmospherePrepared, EARTH, EXTENTS, LUT_PLANES, LUT_SHADER, prepare_compute};
use crate::{atmosphere::AtmosphereFrame, synthetic::SyntheticView};
use std::{fmt::Write, sync::mpsc, time::Duration};
use wgpu::BindingResource::{Sampler, TextureView};

const COLUMNS: [u32; 3] = [32, EXTENTS[1][0], 1024];
const ALTITUDE_ROWS: u32 = 32;
const HEIGHTS: [f32; 2] = [0.0, 245.0];
const OBSERVABLES: [&str; 4] = ["upward_irradiance", "zenith", "antisolar_5_7deg", "local_m"];
const BYTES_PER_VARIANT: u64 = 64;

const OBSERVER: &str = r"
@group(0) @binding(6) var<storage,read_write> observations: array<vec4<f32>>;
var<workgroup> irradiance_lo: array<vec4<f32>,64>;
var<workgroup> irradiance_hi: array<vec4<f32>,64>;
@compute @workgroup_size(64)
fn observe_multiple(@builtin(local_invocation_index) lane: u32) {
    let h=atmosphere.camera_altitude.w;
    let sun=atmosphere.sun_lux.xyz;
    var lo=vec4(0.0);
    var hi=vec4(0.0);
    // Same upward-horizontal quadrature as shipping GroundE, but no GroundE
    // solar-table interpolation, Sky resampling, terminal ground or Sun disk.
    for(var index=lane;index<256u;index+=64u) {
        let sample=ground_hemisphere_sample(index,16u,16u);
        let transport=sky_transport(h,sample.xyz,sun,top_distance(h,sample.y));
        lo+=transport.lo.radiance*sample.w;
        hi+=transport.hi.radiance*sample.w;
    }
    irradiance_lo[lane]=lo;
    irradiance_hi[lane]=hi;
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride {
            irradiance_lo[lane]+=irradiance_lo[lane+stride];
            irradiance_hi[lane]+=irradiance_hi[lane+stride];
        }
        workgroupBarrier();
    }
    if lane==0u {
        observations[0]=vec4(project_spectrum(irradiance_lo[0],irradiance_hi[0]),1.0);
        let zenith=vec3(0.0,1.0,0.0);
        let away=normalize(vec3(-0.995,0.0998,0.0));
        let z=sky_transport(h,zenith,sun,top_distance(h,zenith.y));
        let a=sky_transport(h,away,sun,top_distance(h,away.y));
        let uv=vec2(sun.y*0.5+0.5,h/H);
        let lo=spectral_lut(uv,0u,multiple_scattering,lut_sampler);
        let hi=spectral_lut(uv,1u,multiple_scattering,lut_sampler);
        observations[1]=vec4(project_spectrum(z.lo.radiance,z.hi.radiance),1.0);
        observations[2]=vec4(project_spectrum(a.lo.radiance,a.hi.radiance),1.0);
        observations[3]=vec4(project_spectrum(lo,hi)/MULTIPLE_LUT_GAIN,1.0);
    }
}
";

// Uniform-degree rows plus actual production chart centres and boundaries in
// deep twilight: the uniform sweep alone can miss sub-degree interpolation peaks.
fn sun_degrees() -> Vec<f32> {
    let mut values: Vec<_> = (0_i16..=24).map(|v| -f32::from(v) * 0.5).collect();
    let width = EXTENTS[1][0] as f32;
    for column in 0..EXTENTS[1][0] {
        for offset in [0.0, 0.5] {
            let mu = 2.0 * (column as f32 + offset) / width - 1.0;
            let angle = mu.asin().to_degrees();
            if (-12.0..=-9.0).contains(&angle) {
                values.push(angle);
            }
        }
    }
    values.sort_by(f32::total_cmp);
    values.dedup();
    values
}

struct Variant {
    columns: u32,
    producer: wgpu::ComputePipeline,
    producer_group: wgpu::BindGroup,
    observer_group: wgpu::BindGroup,
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

impl Variant {
    fn new(
        device: &wgpu::Device,
        atmosphere: &AtmospherePrepared,
        observer: &wgpu::ComputePipeline,
        output: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
        columns: u32,
    ) -> Self {
        let texture = if columns == EXTENTS[1][0] {
            atmosphere.textures[1].clone()
        } else {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("same-model M Sun resolution"),
                size: wgpu::Extent3d {
                    width: columns,
                    height: ALTITUDE_ROWS * LUT_PLANES[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let mut views = atmosphere
            .textures
            .each_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        views[1] = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("unchanged production M estimator and encoding"),
            source: wgpu::ShaderSource::Wgsl(EARTH.shader(LUT_SHADER).into()),
        });
        let (producer, producer_group) = if columns == EXTENTS[1][0] {
            (atmosphere.compute[1].clone(), atmosphere.groups[1].clone())
        } else {
            prepare_compute(
                &crate::runtime::DeviceSite::untracked(wgpu::Device::clone(device)),
                1,
                &atmosphere.uniform,
                &views,
                sampler,
                &shader,
                &atmosphere.night,
            )
        };
        let observer_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("same T uniform and sampler for every M width"),
            layout: &observer.get_bind_group_layout(0),
            entries: &[
                entry(0, atmosphere.uniform.as_entire_binding()),
                entry(1, TextureView(&views[0])),
                entry(2, TextureView(&views[1])),
                entry(4, Sampler(sampler)),
                entry(6, output.as_entire_binding()),
            ],
        });
        // Bind groups retain the texture views and their textures.
        Self {
            columns,
            producer,
            producer_group,
            observer_group,
        }
    }

    fn produce(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.producer);
        pass.set_bind_group(0, &self.producer_group, &[]);
        pass.dispatch_workgroups(self.columns.div_ceil(8), ALTITUDE_ROWS.div_ceil(8), 1);
    }
}

fn observer_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shipping sky_transport with test-only physical queries"),
        source: wgpu::ShaderSource::Wgsl(EARTH.shader(&format!("{LUT_SHADER}\n{OBSERVER}")).into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("M interpolation consumer diagnostic"),
        layout: None,
        module: &shader,
        entry_point: Some("observe_multiple"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn collect(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    readback: &wgpu::Buffer,
) -> Vec<[f64; 3]> {
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("M diagnostic receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("bounded M interpolation GPU work completes");
    receive
        .recv()
        .expect("M mapping callback")
        .expect("M observations mapped");
    let result = readback
        .slice(..)
        .get_mapped_range()
        .expect("M diagnostic data")
        .as_chunks::<16>()
        .0
        .iter()
        .map(|record| {
            std::array::from_fn(|channel| {
                let word = record[channel * 4..channel * 4 + 4]
                    .try_into()
                    .expect("f32 record");
                let value = f32::from_le_bytes(word);
                assert!(
                    value.is_finite() && value >= 0.0,
                    "invalid transport observation"
                );
                f64::from(value)
            })
        })
        .collect();
    readback.unmap();
    result
}

fn luminance(rgb: [f64; 3]) -> f64 {
    rgb.into_iter()
        .zip([0.2126, 0.7152, 0.0722])
        .map(|(c, y)| c * y)
        .sum()
}

fn append_rows(report: &mut String, height: f32, sun: f32, rows: &[[f64; 3]]) {
    assert_eq!(rows.len(), COLUMNS.len() * OBSERVABLES.len());
    for (variant, columns) in COLUMNS.into_iter().enumerate() {
        for (quantity, name) in OBSERVABLES.into_iter().enumerate() {
            let value = rows[variant * OBSERVABLES.len() + quantity];
            let dense = rows[(COLUMNS.len() - 1) * OBSERVABLES.len() + quantity];
            // Report zero support as NA, never a fabricated relative error.
            let ratios = std::array::from_fn::<_, 3, _>(|c| {
                if dense[c] > 0.0 {
                    format!("{:.9e}", value[c] / dense[c])
                } else {
                    String::from("NA")
                }
            });
            if columns == EXTENTS[1][0] {
                for (channel, reference) in value.into_iter().zip(dense) {
                    if reference > 0.0 {
                        assert!(
                            (channel / reference - 1.0).abs() <= 0.05,
                            "production M interpolation h={height} sun={sun} {name}: {value:?} vs {dense:?}"
                        );
                    }
                }
            }
            let y = luminance(value);
            let reference_y = luminance(dense);
            let ratio_y = if reference_y > 0.0 {
                format!("{:.9e}", y / reference_y)
            } else {
                String::from("NA")
            };
            writeln!(report,
                "{height}\t{sun}\t{columns}\t{name}\t{:.12e}\t{:.12e}\t{:.12e}\t{y:.12e}\t{}\t{}\t{}\t{ratio_y}",
                value[0], value[1], value[2], ratios[0], ratios[1], ratios[2],
            ).expect("format M diagnostic row");
        }
    }
}

#[test]
fn multiple_sun_columns_report_same_model_transport_and_irradiance_error() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for M interpolation diagnostic");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("M interpolation device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("M diagnostic atmosphere fixture"),
            size: crate::synthetic::LIGHT_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let frame = |sun: f32, height| {
            let (sin, cos) = sun.to_radians().sin_cos();
            AtmosphereFrame::new([cos, sin, 0.0], 1.0, [1.0; 3], height, 1)
                .expect("fixed M diagnostic frame")
        };
        let view = SyntheticView::default();
        let mut atmosphere = AtmospherePrepared::new(
            &crate::runtime::DeviceSite::untracked(wgpu::Device::clone(&device)),
            &queue,
            1,
            frame(0.0, 0.0),
            &view,
            &directional,
            &super::sky_samples(&device),
        );
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("four physical M consumer observations"),
            size: BYTES_PER_VARIANT,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let observer = observer_pipeline(&device);
        let variants = COLUMNS.map(|columns| {
            Variant::new(&device, &atmosphere, &observer, &output, &sampler, columns)
        });
        let mut cold = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = cold.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&atmosphere.compute[0]);
            pass.set_bind_group(0, &atmosphere.groups[0], &[]);
            pass.dispatch_workgroups(EXTENTS[0][0].div_ceil(8), EXTENTS[0][1].div_ceil(8), 1);
        }
        for variant in &variants {
            variant.produce(&mut cold);
        }
        // The submission precedes all observations on this same queue.
        queue.submit([cold.finish()]);
        let mut report = format!(
            "# Same-model M interpolation only; adapter={:?}; widths32,production,1024 x32altitude rows.\n# Unmodified producer, fp16 gain, linear clamp sampler, shared production T.\n# E is upward volume-only irradiance per unit Sun; radiances/M are per-unit-Sun sr^-1.\n# Dense1024 is a representation comparator, not physical truth.\n# Selected chart: <=5% RGB representation error versus1024 on every observed quantity.\n",
            adapter.get_info(),
        );
        report.push_str("height_m\tsun_deg\tcolumns\tquantity\tr\tg\tb\ty\tr_over_1024\tg_over_1024\tb_over_1024\ty_over_1024\n");
        for height in HEIGHTS {
            for sun in sun_degrees() {
                atmosphere.update(&queue, frame(sun, height), &view, [1, 1]);
                let readback = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("M interpolation three-width readback"),
                    size: BYTES_PER_VARIANT * COLUMNS.len() as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                for (index, variant) in variants.iter().enumerate() {
                    {
                        let mut pass =
                            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                        pass.set_pipeline(&observer);
                        pass.set_bind_group(0, &variant.observer_group, &[]);
                        pass.dispatch_workgroups(1, 1, 1);
                    }
                    encoder.copy_buffer_to_buffer(
                        &output,
                        0,
                        &readback,
                        index as u64 * BYTES_PER_VARIANT,
                        BYTES_PER_VARIANT,
                    );
                }
                let rows = collect(&device, &queue, encoder, &readback);
                append_rows(&mut report, height, sun, &rows);
            }
        }
        if let Some(path) = std::env::var_os("THRUM_G3S_MULTIPLE_INTERPOLATION_OUTPUT") {
            std::fs::write(path, report).expect("write explicit M interpolation observations");
        }
        assert!(
            scope.pop().await.is_none(),
            "M interpolation producer and observer validate"
        );
    });
}
