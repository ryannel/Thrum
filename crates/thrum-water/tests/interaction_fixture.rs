#![allow(
    dead_code,
    reason = "shared by the separate interaction and foam GPU checks"
)]
use std::{
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource, SurfaceOutput,
    },
    synthetic::{FrameLighting, OUTPUT_FORMAT, SyntheticScene, SyntheticView},
};
use thrum_sea_model::{Cascade, Spectrum};
use thrum_water::{
    Water, WaterReport, foam,
    interaction::{Limits, Mailbox, Witness, WitnessTap},
    query::{QueryLimits, QueryMailbox},
};
use thrum_water_model::{Bed, Body, BodyKind, Chart, Map, Stage};

const EXTENT: [u32; 2] = [96, 64];
pub(super) const TERRAIN_WIDTH: f32 = 32.0;
const SURFACE: SurfaceProfile = SurfaceProfile {
    format: OUTPUT_FORMAT,
    extent: EXTENT,
};
// Each case uses a separate Metal device and owns several full-grid buffers.
// Keep one live fixture per test process under the shared machine GPU lock.
static INTERACTION_DEVICE: Mutex<()> = Mutex::new(());

pub(super) fn terrain(height: impl Fn(f64) -> f64) -> Map {
    strip(16, 268, height, BodyKind::Sea)
}

pub(super) fn shore() -> Map {
    strip(72, 268, |x| (0.2 * x).max(-5.0), BodyKind::Sea)
}

pub(super) fn current(speed: f64) -> Map {
    let size = [65, 65];
    let count = size[0] * size[1];
    let footprint: Vec<_> = (0..count)
        .map(|index| {
            let [x, z] = [index % size[0], index / size[0]];
            x > 0 && z > 0 && x < size[0] - 1 && z < size[1] - 1
        })
        .collect();
    map(
        Bed::new(
            [-64.0; 2],
            2.0,
            size,
            footprint
                .iter()
                .map(|inside| if *inside { -8.0 } else { 1.0 })
                .collect(),
        )
        .expect("current bed"),
        BodyKind::Chart(Chart {
            levels: vec![0.0; count],
            currents: vec![[speed, 0.0]; count],
            footprint,
            ends: Vec::new(),
        }),
    )
}

pub(super) fn plane_beach() -> Map {
    strip(16, 8, |x| x * 0.10, BodyKind::Sea)
}

fn strip(half_width: i32, half_length: i32, height: impl Fn(f64) -> f64, kind: BodyKind) -> Map {
    let row: Vec<_> = (-half_width..=half_width)
        .map(|x| height(f64::from(x)))
        .collect();
    let dimensions = [
        row.len(),
        usize::try_from(2 * half_length + 1).expect("strip length"),
    ];
    map(
        Bed::new(
            [-f64::from(half_width), -f64::from(half_length)],
            1.0,
            dimensions,
            row.repeat(dimensions[1]),
        )
        .expect("strip bed"),
        kind,
    )
}

fn map(bed: Bed, kind: BodyKind) -> Map {
    Map::prepare(
        bed,
        vec![Body {
            name: "Water".into(),
            kind,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 100_000.0,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 0.8,
            swash_ratio: 0.7,
        },
    )
    .expect("strip water map")
}

struct Clock {
    tick: u64,
    timeline: u64,
}

impl FrameWorld for Clock {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            tick: self.tick,
            timeline: self.timeline,
        }
    }

    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }

    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        let tick = u64::try_from(now.as_nanos() / Fixture::STEP.as_nanos()).expect("test tick");
        let completed_ticks =
            u32::try_from(tick.saturating_sub(self.tick)).expect("short test gap");
        self.tick = tick;
        Ok(WorldAdvance {
            tick,
            completed_ticks,
            overload: None,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }

    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: self.tick,
            },
            timeline: self.timeline,
            page: self.source(),
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn display_phase_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
    fn tick_duration(&self) -> Duration {
        Fixture::STEP
    }
}

struct Source(SceneFrame);
impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            sink.push(0, 1, 1, [0.0; 3], 0);
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.0.view,
            lighting: self.0.lighting,
        })
    }
}

pub(super) struct Fixture {
    gpu: thrum_test_support::Gpu,
    device: wgpu::Device,
    queue: wgpu::Queue,
    host: FrameHost<Clock, Source>,
    target: wgpu::TextureView,
    tap: WitnessTap,
    foam: Option<foam::Tap>,
    pub mailbox: Mailbox,
    pub query: QueryMailbox,
    pub report: Arc<Mutex<WaterReport>>,
    _device_slot: MutexGuard<'static, ()>,
}

impl Fixture {
    pub(super) const STEP: Duration = Duration::from_nanos(16_666_667);

    pub(super) fn new(map: Map, limits: Limits) -> Option<Self> {
        Self::with_fields(map, limits, None)
    }

    pub(super) fn new_foam(
        map: Map,
        wave: Option<thrum_sea_model::Component>,
        wind: [f32; 2],
    ) -> Option<Self> {
        Self::with_fields(
            map,
            Limits {
                bodies: 0,
                impulses: 0,
                solves: 1,
            },
            Some((wave, wind)),
        )
    }

    fn with_fields(
        map: Map,
        limits: Limits,
        foam: Option<(Option<thrum_sea_model::Component>, [f32; 2])>,
    ) -> Option<Self> {
        let device_slot = INTERACTION_DEVICE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let thrum_test_support::Opened {
            gpu, device, queue, ..
        } = thrum_test_support::open(
            "interaction waves",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        )?;
        let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let target = target(&device);
        let view = SyntheticView::perspective(
            [-8.0, 4.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            55_f32.to_radians(),
            1.5,
            0.1,
            500.0,
        )
        .expect("test camera");
        let lighting = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.3, 0.93, 0.2], 100_000.0, [1.0; 3], 55.0, 1)
                .expect("test daylight"),
        );
        let mut host = pollster::block_on(FrameHost::new(
            &caps,
            &queue,
            FrameConfig {
                render_extent: EXTENT,
                surface: Some(SURFACE),
            },
            Clock {
                tick: 0,
                timeline: 1,
            },
            Source(SceneFrame { view, lighting }),
            SyntheticScene::grid(1).expect("scene fixture"),
            1.0,
        ))
        .expect("Metal host");
        host.set_timing_requested(false);
        let spectrum = Spectrum {
            long: foam.and_then(|(wave, _)| wave).into_iter().collect(),
            cascades: [160.0, 37.0, 9.0]
                .map(|length| Cascade::new(length, &[]).expect("calm cascade")),
            tail_slope_variance: 0.0,
        };
        let profile = thrum_water::Profile {
            interaction: Some(limits),
            query: Some(QueryLimits {
                points: 16,
                max_age: 32,
            }),
            ..thrum_water::Profile::default()
        };
        let sea = map
            .body(0)
            .is_some_and(|body| matches!(body.kind, BodyKind::Sea));
        let map = Arc::new(Mutex::new(map));
        let mut water = if sea && foam.is_none() {
            Water::new(map, 0, spectrum, profile)
        } else {
            Water::mapped(map, spectrum, profile)
        }
        .expect("interaction contribution");
        let foam = foam.map(|(_, wind)| {
            water.enable_foam(wind).expect("foam fixture");
            water.tap_foam()
        });
        let mailbox = water.interaction_mailbox().expect("interaction mailbox");
        let query = water.query_mailbox().expect("query mailbox");
        let tap = water.tap_interaction();
        let report = water.report();
        host.set_scene_contribution(Box::new(water))
            .expect("prepare interaction water");
        Some(Self {
            gpu,
            device,
            queue,
            host,
            target,
            tap,
            foam,
            mailbox,
            query,
            report,
            _device_slot: device_slot,
        })
    }

    pub(super) fn draw(&mut self, tick: u64) {
        let output = SurfaceOutput {
            view: &self.target,
            extent: EXTENT,
        };
        assert!(
            self.host
                .drive(
                    Self::STEP * u32::try_from(tick).expect("short test"),
                    Some(&output)
                )
                .expect("drive Metal frame")
                .submitted
        );
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("submitted frame completes");
    }

    pub(super) fn discard(&mut self, tick: u32) {
        drop(
            self.host
                .advance(Self::STEP * tick)
                .expect("advance before discard")
                .encode_scene()
                .expect("discarded scene encoding"),
        );
    }

    pub(super) fn timeline(&mut self, timeline: u64) {
        self.host.world_mut().timeline = timeline;
    }

    pub(super) fn capture(&mut self) {
        self.host.capture_pixels().expect("read-only capture");
    }

    pub(super) fn replace_device(&mut self) {
        let gpu = self.gpu.fresh_adapter();
        let (device, queue) = gpu.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 2);
        let source = self.host.device_prepare_inputs().expect("retain scene");
        let mut prepared = pollster::block_on(source.prepare(&caps, &queue, Some(SURFACE)))
            .expect("new Metal device");
        self.host
            .publish_device(&mut prepared)
            .expect("publish replacement");
        self.target = target(&device);
        self.device = device;
        self.queue = queue;
    }

    pub(super) fn seed_near(&self, state: impl Fn(f64, f64) -> [f32; 2]) {
        let witness = self.tap.latest().expect("seed a submitted calm bank");
        let endpoint = witness.endpoints[1];
        let spacing = f64::from(Witness::CELL_METRES[0]);
        let [x, z] = endpoint.origins[0].map(f64::from);
        let points: Vec<_> = (0..Witness::SIDE.pow(2))
            .map(|i| {
                state(
                    (i % Witness::SIDE) as f64 * spacing + x,
                    (i / Witness::SIDE) as f64 * spacing + z,
                )
            })
            .collect();
        let offset = endpoint.offsets[0] * size_of::<[f32; 2]>();
        self.queue.write_buffer(
            &witness.buffer,
            offset as u64,
            bytemuck::cast_slice(&points),
        );
    }

    pub(super) fn read(&self) -> Field {
        let endpoint = self.tap.latest().expect("submitted endpoints");
        let cells = bytemuck::cast_slice(&thrum_test_support::readback::buffer(
            &self.device,
            &self.queue,
            &endpoint.buffer,
            ..,
        ))
        .to_vec();
        Field { endpoint, cells }
    }

    pub(super) fn witness(&self) -> Option<Witness> {
        self.tap.latest()
    }

    pub(super) fn move_camera(&mut self, x: f32) {
        self.host.continuous_source_mut().0.view = SyntheticView::perspective(
            [x, 4.0, 0.0],
            [x + 9.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            55_f32.to_radians(),
            1.5,
            0.1,
            500.0,
        )
        .expect("shifted camera");
    }

    pub(super) fn foam(&self) -> (foam::Witness, Vec<[f32; 4]>) {
        let witness = self
            .foam
            .as_ref()
            .expect("foam fixture")
            .latest()
            .expect("submitted foam");
        let cells = bytemuck::cast_slice(&thrum_test_support::readback::buffer(
            &self.device,
            &self.queue,
            &witness.buffer,
            ..,
        ))
        .to_vec();
        (witness, cells)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "the 128 cell indices fit the exact f32 integer range"
    )]
    pub(super) fn seed_foam(&self, level: usize, stock: impl Fn([f32; 2]) -> [f32; 4]) {
        let witness = self
            .foam
            .as_ref()
            .expect("foam fixture")
            .latest()
            .expect("submitted foam");
        let side = witness.side;
        let spacing = witness.spacing[level];
        let origin = witness.origins[1][level];
        let cells: Vec<_> = (0..side * side)
            .map(|index| {
                let indices = [index % side, index / side];
                stock(std::array::from_fn(|axis| {
                    (indices[axis] as f32 + 0.5) * spacing + origin[axis]
                }))
            })
            .collect();
        let offset = (2 * witness.banks[1] + level) * cells.len() * size_of::<[f32; 4]>();
        self.queue.write_buffer(
            &witness.buffer,
            u64::try_from(offset).expect("foam plane"),
            bytemuck::cast_slice(&cells),
        );
    }
}

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("interaction test scene"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(super) struct Field {
    pub endpoint: Witness,
    pub cells: Vec<[f32; 2]>,
}

impl Field {
    pub(super) fn level(&self, level: usize) -> &[[f32; 2]] {
        let start = self.endpoint.endpoints[1].offsets[level];
        &self.cells[start..start + Witness::SIDE.pow(2)]
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "coordinates already lie strictly inside the window"
    )]
    pub(super) fn height(&self, point: [f32; 2]) -> f32 {
        let endpoint = self.endpoint.endpoints[1];
        (0..2)
            .map(|level| {
                let [x, z] = std::array::from_fn(|i| {
                    (point[i] - endpoint.origins[level][i]) / Witness::CELL_METRES[level]
                });
                if !(0.0..(Witness::SIDE - 1) as f32).contains(&x)
                    || !(0.0..(Witness::SIDE - 1) as f32).contains(&z)
                {
                    return 0.0;
                }
                let offset = z.floor() as usize * Witness::SIDE + x.floor() as usize;
                let values = self.level(level);
                let lerp = |a: f32, b: f32, blend| a + (b - a) * blend;
                let a = lerp(values[offset][0], values[offset + 1][0], x.fract());
                let b = lerp(
                    values[offset + Witness::SIDE][0],
                    values[offset + Witness::SIDE + 1][0],
                    x.fract(),
                );
                lerp(a, b, z.fract())
            })
            .sum()
    }
}
