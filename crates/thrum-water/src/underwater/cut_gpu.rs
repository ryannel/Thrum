use super::{Plane, RestMesh};
use thrum_render::{
    binding,
    contribution::{
        AfterOpaqueImages, AfterOpaqueKind, AfterOpaquePass, EffectCadence, SceneSurface,
    },
    runtime::DeviceSite,
};

const COVER: u32 = 64;
const CELLS: usize = 65_536;
const CELL_BYTES: u64 = 16;
const FACE_BYTES: u64 = 48;
pub(crate) const OUTPUT: u32 = 22;

#[derive(Clone)]
pub(crate) struct Buffers {
    input: wgpu::Buffer,
    pub(crate) output: wgpu::Buffer,
}

impl Buffers {
    pub(crate) fn new(device: &DeviceSite) -> Self {
        let create = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Self {
            input: create("lens rest cells", 32 + CELLS as u64 * CELL_BYTES),
            output: create("lens displaced faces", 16 + 2 * CELLS as u64 * FACE_BYTES),
        }
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
        mesh: &wgpu::Buffer,
        entries: &[wgpu::BindGroupLayoutEntry],
        bindings: &[wgpu::BindGroupEntry<'_>],
    ) -> Self {
        let device = surface.device;
        let stages = wgpu::ShaderStages::COMPUTE;
        let field_entries: Vec<_> = entries
            .iter()
            .filter(|entry| entry.visibility.contains(wgpu::ShaderStages::VERTEX))
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
            entries: &[
                binding::storage(23, stages, true, 0),
                binding::storage(24, stages, true, 0),
                binding::storage(25, stages, false, 0),
            ],
        });
        let work = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lens cut input and output"),
            layout: &work_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 23,
                    resource: mesh.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 24,
                    resource: buffers.input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 25,
                    resource: buffers.output.as_entire_binding(),
                },
            ],
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
    ) -> bool {
        self.cells.clear();
        let plane = Plane::new(frame);
        let bounds = plane.bounds();
        let anchor = [frame.camera[0].floor(), frame.camera[2].floor()].map(f64::from);
        if admitted && horizontal.is_finite() {
            let [lo, hi] = bounds;
            let expanded = [lo.map(|v| v - horizontal), hi.map(|v| v + horizontal)];
            self.cells
                .extend(mesh.cells(expanded, anchor, hollows).take(CELLS + 1));
        }
        let valid = admitted && horizontal.is_finite() && self.cells.len() <= CELLS;
        if !valid {
            self.cells.clear();
        }
        let count = u32::try_from(self.cells.len()).expect("bounded lens cells");
        queue.write_buffer(
            &self.buffers.output,
            0,
            bytemuck::bytes_of(&[0u32, u32::from(valid), 0, 0]),
        );
        if count > 0 {
            queue.write_buffer(
                &self.buffers.input,
                0,
                bytemuck::bytes_of(&[count, 0, 0, 0]),
            );
            queue.write_buffer(
                &self.buffers.input,
                16,
                bytemuck::bytes_of(&bounds.map(|p| p.map(|v| v as f32))),
            );
            queue.write_buffer(&self.buffers.input, 32, bytemuck::cast_slice(&self.cells));
        }
        valid
    }

    pub(crate) fn runs(&self) -> bool {
        !self.cells.is_empty()
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.work, &[]);
        pass.set_bind_group(3, &self.field, &[]);
        pass.dispatch_workgroups(
            u32::try_from(self.cells.len())
                .expect("bounded lens cells")
                .div_ceil(COVER),
            1,
            1,
        );
    }
}
