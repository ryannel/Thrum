#![allow(dead_code, reason = "map and gains run as separate item commands")]
use thrum_water::map_gpu;
use thrum_water_model::{Bed, Body, BodyKind, Chart, Map, Profile, Stage};
use wgpu::util::DeviceExt;

pub(crate) const EPS_METRES: f64 = 0.00004;
pub(crate) const EPS_CURRENT: f64 = 0.000_004;

pub(crate) fn probe(
    gpu: &thrum_test_support::Opened,
    map: &Map,
    inputs: &[[f32; 8]],
) -> Vec<[[f32; 4]; 3]> {
    let thrum_test_support::Opened { device, queue, .. } = gpu;
    let map = map_gpu::MapGpu::new(device, queue, map).expect("map products");
    let view = map.view();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("water map probe points"),
        contents: bytemuck::cast_slice(inputs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (size_of::<[f32; 12]>() * inputs.len()) as u64;
    let buffer = |label, usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let output = buffer(
        "water map probe answers",
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let source = map.source(0) + include_str!("map_probe.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("water map proof"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("water map proof"),
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: input.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: output.as_entire_binding(),
        },
    ];
    entries.extend(map.entries(&view));
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("water map probe bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(inputs.len().div_ceil(64)).expect("points"),
            1,
            1,
        );
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &output, 0..size);
    let submission = queue.submit([encoder.finish()]);
    bytemuck::cast_slice(&readback.wait(device, submission)).to_vec()
}

pub(crate) fn body(name: &str, kind: BodyKind, level: f64, fetch: f64) -> Body {
    Body {
        name: name.into(),
        kind,
        level,
        fetch,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        gains: [1.0; 4],
    }
}

pub(crate) fn fixture() -> Map {
    let [nx, nz] = [77, 68];
    let mut heights = Vec::new();
    let mut levels = Vec::new();
    let mut currents = Vec::new();
    let mut footprint = Vec::new();
    for j in 0..nz {
        for i in 0..nx {
            let x = i as f64;
            let z = j as f64;
            let is_chart = (29..=61).contains(&i) && (51..=57).contains(&j);
            let level = 3.2 - 0.028 * (x - 29.0) + 0.021 * (z - 54.0);
            footprint.push(is_chart);
            levels.push(level);
            currents.push([1.0 + x * 0.023, 0.5 - z * 0.008]);
            let h = (-1.93 + x * 0.21 + z * 0.004 + 0.7 * (x - 11.0).max(0.0))
                .min(11.8)
                .min(2.0 + 0.39 * (x - 27.1).hypot(z - 20.4))
                .min(4.2 + 0.41 * (x - 56.0).hypot(z - 30.0));
            heights.push(if is_chart {
                level - 0.78 + 0.032 * (z - 54.0).abs()
            } else {
                h
            });
        }
    }
    Map::prepare(
        Bed::new([0.0, 0.0], 1.0, [nx, nz], heights).expect("bed"),
        vec![
            body("sea", BodyKind::Sea, 0.0, 100_000.0),
            body(
                "lake",
                BodyKind::Basin { seed: [27.1, 20.4] },
                4.95,
                1_000.0,
            ),
            body("pond", BodyKind::Basin { seed: [56.0, 30.0] }, 6.75, 130.0),
            body(
                "flow",
                BodyKind::Chart(Chart {
                    levels,
                    currents,
                    footprint,
                    ends: Vec::new(),
                }),
                0.0,
                400.0,
            ),
        ],
        Profile {
            margin_height: 1.1,
            swash_ratio: 0.7,
        },
    )
    .expect("valid map")
}
