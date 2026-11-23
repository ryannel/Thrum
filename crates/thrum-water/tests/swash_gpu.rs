//! A plane beach, its independent CPU waterline and the submitted depth cut.
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    abi::InstanceRecord,
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource, SurfaceOutput,
    },
    synthetic::{
        FrameLighting, OUTPUT_FORMAT, SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes,
        SyntheticScene, SyntheticVertex, SyntheticView,
    },
};
use thrum_sea_model::{Cascade, Component, LongWaves, Spectrum};
use thrum_water::Water;
use thrum_water_model::{Bed, Body, BodyKind, Field, Map, Profile, Stage};

const EXTENT: [u32; 2] = [640, 192];
const HEIGHT_SLOPE: f64 = 0.1;
const MAP_TOLERANCE_METRES: f64 = 0.00004;

pub(crate) struct Clock {
    tick: u64,
    interpolate: bool,
    now: Duration,
}

impl Clock {
    pub(crate) fn new(interpolate: bool) -> Self {
        Self {
            tick: 0,
            interpolate,
            now: Duration::ZERO,
        }
    }
}

impl FrameWorld for Clock {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            tick: self.tick,
            timeline: 1,
        }
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        self.now = now;
        let next =
            u64::try_from(now.as_nanos() / self.tick_duration().as_nanos()).expect("short test");
        let completed_ticks = u32::try_from(next - self.tick).expect("short test");
        self.tick = next;
        Ok(WorldAdvance {
            tick: next,
            overload: None,
            completed_ticks,
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
            timeline: 1,
            page: self.source(),
        })
    }
    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        if self.interpolate {
            let step = self.tick_duration().as_nanos();
            let part = self.now.as_nanos() % step + lead.as_nanos();
            part.min(step) as f32 / step as f32
        } else {
            1.0
        }
    }
    fn display_phase_at(&self, lead: Duration) -> f32 {
        self.presentation_alpha_at(lead)
    }
    fn is_interpolating(&self) -> bool {
        self.interpolate
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

pub(crate) struct Source {
    pub(crate) view: SyntheticView,
    pub(crate) lighting: FrameLighting,
}
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
            view: self.view,
            lighting: self.lighting,
        })
    }
}

pub(crate) fn beach(ratio: f64, max_stage: f64) -> Map {
    let heights = (0_i16..7)
        .flat_map(|_| (-48_i16..=48).map(|x| f64::from(x) * HEIGHT_SLOPE))
        .collect();
    Map::prepare(
        Bed::new([-48.0, -3.0], 1.0, [97, 7], heights).expect("beach grid"),
        vec![Body {
            name: "sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: max_stage,
                value: 0.0,
            },
            fetch: 10_000.0,
            gains: [1.0; 4],
        }],
        Profile {
            margin_height: 2.0,
            swash_ratio: ratio,
        },
    )
    .expect("plane map")
}

fn plane() -> SyntheticMesh {
    SyntheticMesh {
        vertices: [
            [-48.0, -4.8, -3.0],
            [-48.0, -4.8, 3.0],
            [48.0, 4.8, -3.0],
            [48.0, 4.8, 3.0],
        ]
        .into_iter()
        .map(|position| SyntheticVertex {
            position,
            normal: [-0.099_504, 0.995_037, 0.0],
            uv: [0.0; 2],
            tangent: [1.0, 0.1, 0.0, 1.0],
        })
        .collect(),
        indices: vec![0, 1, 2, 2, 1, 3],
    }
}

fn read_row(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<[u16; 4]> {
    // The middle row of eight-byte texels, sliced from the packed mip.
    let start = u64::from(EXTENT[1] / 2) * u64::from(texture.width()) * 8;
    let size = u64::from(EXTENT[0]) * 8;
    bytemuck::cast_slice(&thrum_test_support::readback::texture(
        device,
        queue,
        texture,
        thrum_test_support::readback::Mip::level(0),
        start..start + size,
    ))
    .to_vec()
}

fn plane_scanline(view: &SyntheticView) -> Vec<[f64; 2]> {
    let inverse = glam::Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
    (0..EXTENT[0])
        .map(|x| {
            let ndc_x = (x as f32 + 0.5) * 2.0 / EXTENT[0] as f32 - 1.0;
            let ndc_y = 1.0 - (EXTENT[1] as f32 + 1.0) / EXTENT[1] as f32;
            let point = |depth| {
                inverse
                    .project_point3(glam::vec3(ndc_x, ndc_y, depth))
                    .to_array()
                    .map(f64::from)
            };
            let origin = point(1.0);
            let far = point(0.01);
            let ray: [f64; 3] = std::array::from_fn(|i| far[i] - origin[i]);
            let t = (HEIGHT_SLOPE * origin[0] - origin[1]) / (ray[1] - HEIGHT_SLOPE * ray[0]);
            [origin[0] + ray[0] * t, origin[2] + ray[2] * t]
        })
        .collect()
}

#[test]
fn submitted_depth_waterline_matches_the_cpu_wave_at_display_time() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "swash depth",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let surface = SurfaceProfile {
        format: OUTPUT_FORMAT,
        extent: EXTENT,
    };
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("swash proof output"),
        size: wgpu::Extent3d {
            width: EXTENT[0],
            height: EXTENT[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = SyntheticView::perspective(
        [0.0, 19.0, 0.0],
        [0.0, 0.0, -0.001],
        [0.0, 1.0, 0.0],
        30_f32.to_radians(),
        EXTENT[0] as f32 / EXTENT[1] as f32,
        0.1,
        1000.0,
    )
    .expect("top-down camera")
    .with_temporal_inputs(true);
    let ground_points = plane_scanline(&view);
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.3, 0.93, 0.2], 100_000.0, [1.0; 3], 55.0, 1).expect("noon"),
    );
    let mut scene = SyntheticScene::grid(1).expect("fixture");
    scene.view = view;
    scene.instances[0] = InstanceRecord::identity(0, 0);
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        meshes: vec![plane()],
        slots: vec![SyntheticMeshChoice::Mesh(0)],
        updates: None,
    }));
    let mut host = pollster::block_on(FrameHost::new(
        &caps,
        &queue,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(surface),
        },
        Clock::new(false),
        Source { view, lighting },
        scene,
        1.0,
    ))
    .expect("ordinary frame host");
    let target = output.create_view(&wgpu::TextureViewDescriptor::default());
    let surface_output = SurfaceOutput {
        view: &target,
        extent: EXTENT,
    };
    let submit = |host: &mut FrameHost<Clock, Source>, time| {
        host.set_timing_requested(true);
        assert!(
            host.drive(Duration::from_secs_f64(time), Some(&surface_output))
                .expect("drive")
                .submitted
        );
        let texture = host
            .submitted_temporal_witness()
            .expect("submitted reactive and depth")
            .clone();
        let sample =
            thrum_test_support::wait::until("completed frame", Duration::from_secs(10), || {
                device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("frame");
                host.poll_completed().expect("ready")
            });
        (sample, read_row(&device, &queue, &texture))
    };
    drop(submit(&mut host, 0.0));
    let (_, ground) = submit(&mut host, 1.0 / 60.0);
    assert!(
        ground.iter().all(|pixel| pixel[0] == 0 && pixel[1] > 0),
        "the entire scanline lies on opaque sand; first {:?}, centre {:?}, last {:?}, opaque depth {}",
        ground[0],
        ground[ground.len() / 2],
        ground[ground.len() - 1],
        ground.iter().filter(|p| p[1] > 0).count(),
    );
    let wave = Component {
        amplitude: 0.5,
        wave_vector: [std::f64::consts::TAU / 64.0, 0.0],
        phase: 0.0,
    };
    let field = Field {
        layers: std::array::from_fn(|i| {
            LongWaves::new(if i == 0 { vec![wave] } else { vec![] }, 0.0).expect("oracle wave")
        }),
    };
    let period = std::f64::consts::TAU / wave.frequency();
    let mut elapsed = 1.0 / 60.0;
    for ratio in [0.7, 0.0] {
        let map = Arc::new(Mutex::new(beach(ratio, 0.0)));
        let spectrum = Spectrum {
            long: vec![wave],
            cascades: [160.0, 37.0, 9.0]
                .map(|length| Cascade::new(length, &[]).expect("quiet band")),
            tail_slope_variance: 0.0,
        };
        let mut profile = thrum_water::Profile::default();
        profile.waves.choppiness = 0.0;
        let water = Water::mapped(Arc::clone(&map), spectrum, profile).expect("wave sheet");
        host.set_scene_contribution(Box::new(water))
            .expect("surface");
        let mut edges = Vec::new();
        for _ in 0..32 {
            elapsed += period / 32.0;
            let (frame, pixels) = submit(&mut host, elapsed);
            let time = (frame.tick as f64 - 1.0 + f64::from(frame.presentation_alpha))
                * Duration::from_secs_f64(1.0 / 60.0).as_secs_f64();
            let cpu = map.lock().expect("model");
            let z = ground_points[ground_points.len() / 2][1];
            let mut lo = -6.0;
            let mut hi = 6.0;
            for _ in 0..42 {
                let x = f64::midpoint(lo, hi);
                if cpu
                    .surface(&field, [x, z], time)
                    .expect("CPU wet support")
                    .wet
                {
                    lo = x;
                } else {
                    hi = x;
                }
            }
            let expected = f64::midpoint(lo, hi);
            // Opaque motion may be invalid across the intentionally spaced
            // samples, in which case its reactive word is 1. At this steep
            // view the flat-wave water itself has a sub-unit Fresnel mask.
            let is_sheet = |water: &[u16; 4], sand: &[u16; 4]| {
                water[0] > 0 && water[0] < 0x3c00 && water[1] >= sand[1]
            };
            let edge = pixels
                .iter()
                .zip(&ground)
                .rposition(|(water, sand)| is_sheet(water, sand))
                .expect("drawn edge");
            assert!(edge + 1 < pixels.len(), "scanline crosses the waterline");
            assert!(
                pixels
                    .iter()
                    .zip(&ground)
                    .take(edge)
                    .all(|(water, sand)| is_sheet(water, sand)),
                "one continuous depth-writing sheet"
            );
            assert!(
                pixels
                    .iter()
                    .zip(&ground)
                    .skip(edge + 1)
                    .all(|(water, sand)| water[1] == sand[1]),
                "dry ground retains its depth"
            );
            let actual = ground_points[edge][0].midpoint(ground_points[edge + 1][0]);
            let pixel = ground_points[edge + 1][0] - ground_points[edge][0];
            assert!(
                (expected - actual).abs() <= pixel + MAP_TOLERANCE_METRES,
                "ratio={ratio} t={time:.4} CPU={expected:.4} drawn={actual:.4} pixel={pixel:.4}"
            );
            edges.push(edge);
        }
        let travel = edges.iter().max().expect("frames") - edges.iter().min().expect("frames");
        if ratio > 0.0 {
            assert!(travel > 50, "moving waterline: {travel} px");
        } else {
            assert_eq!(travel, 0, "zero swash has a fixed waterline");
        }
    }
}
