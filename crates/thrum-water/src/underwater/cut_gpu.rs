use super::{Plane, RestMesh};
use thrum_render::{
    binding,
    contribution::{
        AfterOpaqueImages, AfterOpaqueKind, AfterOpaquePass, EffectCadence, SceneSurface,
    },
    runtime::DeviceSite,
};

const COVER: u32 = 64;
const FACES: u64 = 32_768;
const CELL_BYTES: u64 = 16;
const FACE_BYTES: u64 = 64;
pub(crate) const OUTPUT: u32 = 22;
const MESH: u32 = 23;
const INPUT: u32 = 24;

#[derive(Clone)]
pub(crate) struct Buffers {
    input: wgpu::Buffer,
    output: wgpu::Buffer,
}

impl Buffers {
    pub(crate) fn new(
        device: &DeviceSite,
        rest: &RestMesh,
        vertex_bytes: u64,
    ) -> Result<Self, String> {
        let create = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let input_bytes = 32 + rest.cell_capacity() * CELL_BYTES;
        let output_bytes = 16 + FACES * FACE_BYTES;
        if input_bytes.max(output_bytes).max(vertex_bytes)
            > device.limits().max_storage_buffer_binding_size
        {
            return Err("the lens mesh exceeds the device's storage binding size".into());
        }
        Ok(Self {
            input: create("lens rest cells", input_bytes),
            output: create("lens displaced faces", output_bytes),
        })
    }

    pub(crate) fn render_bindings() -> [wgpu::BindGroupLayoutEntry; 3] {
        [OUTPUT, MESH, INPUT]
            .map(|slot| binding::storage(slot, wgpu::ShaderStages::FRAGMENT, true, 0))
    }

    pub(crate) fn render_entries<'a>(
        &'a self,
        mesh: &'a wgpu::Buffer,
    ) -> [wgpu::BindGroupEntry<'a>; 3] {
        [(OUTPUT, &self.output), (MESH, mesh), (INPUT, &self.input)].map(|(binding, buffer)| {
            wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }
        })
    }
}

pub(crate) struct Cut {
    buffers: Buffers,
    field: wgpu::BindGroup,
    work: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    cells: Vec<[u32; 4]>,
}

impl Cut {
    pub(crate) fn output(&self) -> &wgpu::Buffer {
        &self.buffers.output
    }

    pub(crate) fn render_entries<'a>(
        &'a self,
        mesh: &'a wgpu::Buffer,
    ) -> [wgpu::BindGroupEntry<'a>; 3] {
        self.buffers.render_entries(mesh)
    }

    pub(crate) fn resource_bytes(&self) -> [(String, u64); 2] {
        [
            ("lens rest cells".into(), self.buffers.input.size()),
            ("lens displaced faces".into(), self.buffers.output.size()),
        ]
    }
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        shader: &wgpu::ShaderModule,
        buffers: Buffers,
        entries: &[wgpu::BindGroupLayoutEntry],
        bindings: &[wgpu::BindGroupEntry<'_>],
    ) -> Self {
        let device = surface.device;
        let stages = wgpu::ShaderStages::COMPUTE;
        let field_entries: Vec<_> = entries
            .iter()
            .filter(|entry| {
                entry.visibility.contains(wgpu::ShaderStages::VERTEX)
                    || [MESH, INPUT].contains(&entry.binding)
            })
            .map(|entry| wgpu::BindGroupLayoutEntry {
                visibility: stages,
                ..*entry
            })
            .collect();
        let field_bindings: Vec<_> = bindings
            .iter()
            .filter(|entry| {
                field_entries
                    .iter()
                    .any(|layout| layout.binding == entry.binding)
            })
            .cloned()
            .collect();
        let field_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lens vertex fields"),
            entries: &field_entries,
        });
        let field = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lens vertex fields"),
            layout: &field_layout,
            entries: &field_bindings,
        });
        let work_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lens cut input and output"),
            entries: &[binding::storage(25, stages, false, 0)],
        });
        let work = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lens cut input and output"),
            layout: &work_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 25,
                resource: buffers.output.as_entire_binding(),
            }],
        });
        let inputs = Self::declaration().input_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water lens cut"),
            immediate_size: 0,
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&inputs),
                Some(&work_layout),
                Some(&field_layout),
            ],
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("water lens cut"),
            layout: Some(&layout),
            module: shader,
            entry_point: Some("displace_lens_faces"),
            cache: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });
        Self {
            buffers,
            field,
            work,
            pipeline,
            cells: Vec::new(),
        }
    }

    pub(crate) fn declaration() -> AfterOpaquePass {
        AfterOpaquePass {
            name: "water lens cut".into(),
            kind: AfterOpaqueKind::Compute,
            cadence: EffectCadence::Skippable,
            accesses: Vec::new(),
            depth: false,
            write_depth: false,
            refraction: false,
            images: AfterOpaqueImages::None,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "lens corners originate in f32"
    )]
    pub(crate) fn record(
        &mut self,
        queue: &wgpu::Queue,
        frame: &thrum_render::contribution::ContributionFrame,
        mesh: &RestMesh,
        admitted: bool,
        horizontal: f64,
        hollows: Option<&crate::prepare::hollow_tiles::HollowTiles>,
    ) {
        self.cells.clear();
        let plane = Plane::new(frame);
        let bounds = plane.bounds();
        let anchor = [frame.camera[0].floor(), frame.camera[2].floor()].map(f64::from);
        if admitted {
            let [lo, hi] = bounds;
            let expanded = [lo.map(|v| v - horizontal), hi.map(|v| v + horizontal)];
            self.cells.extend(mesh.cells(expanded, anchor, hollows));
        }
        let count = u32::try_from(self.cells.len()).expect("bounded lens cells");
        queue.write_buffer(
            &self.buffers.output,
            0,
            bytemuck::bytes_of(&[0u32, u32::from(admitted), 0, 0]),
        );
        if count > 0 {
            queue.write_buffer(
                &self.buffers.input,
                0,
                bytemuck::bytes_of(&[count, count.div_ceil(COVER).min(256) * COVER, 0, 0]),
            );
            queue.write_buffer(
                &self.buffers.input,
                16,
                bytemuck::bytes_of(&bounds.map(|p| p.map(|v| v as f32))),
            );
            queue.write_buffer(&self.buffers.input, 32, bytemuck::cast_slice(&self.cells));
        }
    }

    pub(crate) fn runs(&self) -> bool {
        !self.cells.is_empty()
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.work, &[]);
        pass.set_bind_group(3, &self.field, &[]);
        let groups = u32::try_from(self.cells.len())
            .expect("bounded lens cells")
            .div_ceil(COVER);
        let width = groups.min(256);
        pass.dispatch_workgroups(width, groups.div_ceil(width), 1);
    }
}
