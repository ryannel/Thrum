//! A short perspective ray, with an off-axis caster and a sampled Sun path.
#![expect(
    dead_code,
    reason = "the three planned GPU test targets use different observations"
)]

use thrum_render::{
    abi::{InstanceRecord, MaterialRecord},
    atmosphere::{Aerosol, AtmosphereFrame, SunOpticalDepth},
    capabilities::DeviceCapabilities,
    synthetic::{
        CaptureSource, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene,
        SyntheticView,
    },
};
use thrum_test_support::readback::{Mip, Readback};

pub(crate) const EXTENT: [u32; 2] = [33, 19];
pub(crate) const SUN_LUX: f32 = 127_000.0;
pub(crate) const STORAGE_EXPOSURE: f32 = 1.0 / 1024.0;
pub(crate) const SCATTER_GAIN: f32 = 1024.0;

pub(crate) struct Gpu {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) caps: DeviceCapabilities,
}

impl Gpu {
    pub(crate) fn open(generation: u64, label: &str) -> Option<Self> {
        let test = thrum_test_support::open(
            label,
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        )?;
        assert_eq!(test.gpu.adapter.get_info().backend, wgpu::Backend::Metal);
        let caps = DeviceCapabilities::capture(&test.gpu.adapter, &test.device, generation);
        Some(Self {
            device: test.device,
            queue: test.queue,
            caps,
        })
    }

    pub(crate) fn prepare(
        &self,
        scene: &SyntheticScene,
        light: AtmosphereFrame,
    ) -> (PreparedSynthetic, OffscreenTarget) {
        let prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
            &self.caps,
            &self.queue,
            scene,
            FrameLighting::Atmosphere(light),
            1.0,
        ))
        .expect("prepare scene on Metal");
        let target = OffscreenTarget::with_capture_source(
            &self.device,
            &prepared,
            EXTENT[0],
            EXTENT[1],
            CaptureSource::SceneLinear,
        )
        .expect("scene-linear target");
        (prepared, target)
    }
}

pub(crate) fn view(eye: [f32; 3]) -> SyntheticView {
    SyntheticView::perspective(
        eye,
        [eye[0], eye[1], eye[2] - 1.0],
        [0.0, 1.0, 0.0],
        0.75,
        33.0 / 19.0,
        0.1,
        2000.0,
    )
    .expect("checked camera")
    .with_exposure(0.001)
    .expect("fixed exposure")
}

pub(crate) fn light(volume: bool, epoch: u64) -> AtmosphereFrame {
    let frame = AtmosphereFrame::new([0.985, 0.173, 0.0], SUN_LUX, [1.0; 3], 2.0, epoch)
        .expect("Sun")
        .with_nightglow()
        .with_aerosol(Aerosol::new(0.000_184, 0.0).expect("haze"));
    if volume {
        frame.with_view_volume()
    } else {
        frame
    }
}

fn cube(slot: u32, center: [f32; 3], scale: [f32; 3]) -> InstanceRecord {
    let mut item = InstanceRecord::identity(slot, 0);
    for (axis, value) in scale.into_iter().enumerate() {
        item.current[axis][axis] = value;
        item.normal[axis][axis] = value.recip();
        item.current[3][axis] = center[axis];
    }
    item.previous = item.current;
    item
}

pub(crate) fn scene(caster: bool, cloud: bool, receiver: bool) -> SyntheticScene {
    let mut scene = SyntheticScene::grid(2).expect("small world");
    scene.view = view([0.0, 2.0, 0.0]);
    scene.sun_shadows = caster;
    scene.materials[0] = MaterialRecord {
        base_rgba: [0.0, 0.0, 0.0, 1.0],
        surface: [1.0, 1.0, 1.0, 1.0],
        ..MaterialRecord::default()
    };
    scene.instances[0] = if receiver {
        cube(0, [14.0, 4.45, -24.0], [2.0, 4.0, 30.0])
    } else {
        // The sky falsifiers need a difference larger than the stored pixel's
        // rounding tolerance. Extend the shadow in depth, away from the ray.
        cube(0, [14.0, 4.45, -235.0], [4.0, 9.0, 400.0])
    };
    scene.instances[1] = if receiver {
        cube(1, [0.0, 2.0, -28.25], [1.0, 1.0, 0.5])
    } else {
        cube(1, [-30.0, 2.0, 10.0], [1.0; 3])
    };
    if cloud {
        // The field intercept is about (17 m, z) at the centre of the volume.
        // It shadows the central samples, with a clear region beyond 55 m.
        scene.sun_optical_depth = Some(sun_path(1.0, if receiver { -55.0 } else { -660.0 }));
    }
    scene
}

pub(crate) fn sun_path(depth: f32, end_z: f32) -> SunOpticalDepth {
    SunOpticalDepth::new([2, 2], &[depth; 4], 5.0, [-20.0, end_z, 55.0, 4.0], 0.01, 1)
        .expect("sampled Sun path")
}

pub(crate) struct Witness {
    pub(crate) image: Vec<u8>,
    pub(crate) scatter: Vec<u8>,
    pub(crate) transmission: Vec<u8>,
    pub(crate) grid: [u32; 3],
}

impl Witness {
    pub(crate) fn center(&self) -> [f32; 4] {
        pixel(&self.image, EXTENT[0], EXTENT[0] / 2, EXTENT[1] / 2)
    }

    pub(crate) fn terminal(&self) -> [f32; 4] {
        let pixels = (self.grid[0] * self.grid[1]) as usize * 8;
        self.volume_center(&self.scatter[pixels * (self.grid[2] as usize - 1)..])
    }

    pub(crate) fn volume_center(&self, layer: &[u8]) -> [f32; 4] {
        // Both volume axes have even sizes. The centre pixel samples the
        // arithmetic mean of these four half-float texels with the linear sampler.
        let x = self.grid[0] / 2;
        let y = self.grid[1] / 2;
        let samples = [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)]
            .map(|(x, y)| pixel(layer, self.grid[0], x, y));
        std::array::from_fn(|lane| samples.iter().map(|p| p[lane]).sum::<f32>() * 0.25)
    }
}

pub(crate) fn pixel(bytes: &[u8], width: u32, x: u32, y: u32) -> [f32; 4] {
    let start = (y * width + x) as usize * 8;
    std::array::from_fn(|lane| {
        let offset = start + 2 * lane;
        half::f16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("texel lane")).to_f32()
    })
}

pub(crate) fn render(
    gpu: &Gpu,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    light: AtmosphereFrame,
) -> Witness {
    let generation = gpu.caps.generation();
    prepared
        .update_frame(
            &gpu.queue,
            target,
            scene,
            FrameLighting::Atmosphere(light),
            1.0,
        )
        .expect("live frame");
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut image = target.readback(&gpu.device);
    prepared
        .encode_with_readback(&mut encoder, generation, target, None, &mut image)
        .expect("encode scene-linear readback");
    let volume = prepared
        .view_volume_witness()
        .map(|([scatter, transmission], _)| {
            let dimensions = [
                scatter.width(),
                scatter.height(),
                scatter.depth_or_array_layers(),
            ];
            let reads = [scatter, transmission].map(|texture| {
                Readback::texture(&gpu.device, &mut encoder, texture, Mip::default(), ..)
            });
            (dimensions, reads)
        });
    let submission = gpu.queue.submit([encoder.finish()]);
    prepared.commit_frame();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission.clone()),
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("submit on Metal");
    let image = image
        .try_collect(generation)
        .expect("completed map")
        .expect("image");
    let (grid, [scatter, transmission]) = volume.map_or_else(
        || ([0, 0, 0], [Vec::new(), Vec::new()]),
        |(grid, reads)| {
            (
                grid,
                reads.map(|read| read.wait(&gpu.device, submission.clone())),
            )
        },
    );
    Witness {
        image,
        scatter,
        transmission,
        grid,
    }
}
