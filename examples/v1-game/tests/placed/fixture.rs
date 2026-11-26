use std::{fs, sync::Arc, time::Duration};

use thrum_asset::Profile;
use thrum_cloud_model::{
    Directed, Parameters, PlacedLibrary, PlacedSlab, Shape, ShapeSource, State,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, encode_volume};
use thrum_render::runtime::{FrameConfig, FrameHost, SurfaceOutput};
use thrum_test_support::{Opened, Scratch};
use v1_game::{CloudSelection, GameHost, GameScene, GameSetup, GameWorld, PlacedSky};

pub(super) type Host = FrameHost<GameWorld, GameScene>;
pub(super) const EXTENT: [u32; 2] = [96, 64];
pub(super) const TEXEL: f64 = 80.0;
const CENTRES: [[f64; 2]; 2] = [[20.0, -1_100.0], [0.0, -1_980.0]];

pub(super) struct Fixture {
    pub(super) state: State,
    pub(super) sky: PlacedSky,
    cooker: Cooker,
    source: Scratch,
}

impl Fixture {
    pub(super) fn new(resident: bool) -> Self {
        Self::with_placements(resident, &CENTRES, [0.0; 2])
    }

    pub(super) fn with_placements(resident: bool, centres: &[[f64; 2]], wind: [f64; 2]) -> Self {
        Self::with_coverage(resident, centres, wind, 0.0)
    }

    pub(super) fn with_coverage(
        resident: bool,
        centres: &[[f64; 2]],
        wind: [f64; 2],
        coverage: f64,
    ) -> Self {
        let source = Scratch::new("game-placed");
        let inputs = source.join("source");
        fs::create_dir(&inputs).expect("source");
        let volume: Vec<u8> = (0..512)
            .map(|index| {
                let x = index % 8;
                let y = (index / 8) % 8;
                let z = index / 64;
                if (1..7).contains(&x) && (2..6).contains(&z) && (1..x).contains(&y) {
                    128
                } else {
                    0
                }
            })
            .collect();
        let source_bytes = encode_volume(8, 8, 8, &volume);
        fs::write(inputs.join("cloud-library.tvol"), &source_bytes).expect("library stack");
        let mut growth_bytes = source_bytes;
        growth_bytes[16] ^= 1;
        fs::write(inputs.join("cloud-growth.tvol"), &growth_bytes).expect("distinct growth stack");
        let store = source.join("store");
        let mut cooker = Cooker::open(CookConfig {
            source: inputs,
            store: store.clone(),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("cooker");
        if resident {
            assert_eq!(cooker.cook_all().expect("cook").failed, []);
        }
        let shape = Shape::new(
            [8; 3],
            60.0,
            volume.iter().map(|&v| f64::from(v) / 255.0).collect(),
        )
        .expect("headless shape");
        let footprint = Arc::new(shape.footprint().clone());
        let slab = PlacedSlab {
            extent: 480.0,
            floor: 0.0,
            heading: 0.0,
            footprint: Arc::clone(&footprint),
        };
        let library =
            Arc::new(PlacedLibrary::new(vec![slab.clone()], vec![slab]).expect("library"));
        let mut state = State::new(
            Parameters {
                coverage,
                kind: 1.0,
                wind,
                time_scale: 60.0,
            },
            0.5,
            16,
        )
        .expect("weather");
        state.set_library(library).expect("library in game weather");
        let shape_at = |id, centre| Directed {
            id,
            centre,
            yaw: 0.0,
            scale: 1.0,
            footprint: Arc::clone(&footprint),
            envelope: 0.0,
            target: 1.0,
            growth: 0.0,
            growth_target: 0.0,
        };
        for (id, &centre) in centres.iter().enumerate() {
            state
                .add_from_library(
                    shape_at([2, 1][id], centre),
                    if id == 1 {
                        ShapeSource::Growth
                    } else {
                        ShapeSource::Library(0)
                    },
                )
                .expect("admitted directed shape");
        }
        assert_eq!(
            state.add_from_library(shape_at(4, centres[0]), ShapeSource::Library(0)),
            Err(thrum_cloud_model::Error::Overlap)
        );
        let mut profile = v1_game::clouds::shade_config();
        profile.weather.size = 64;
        profile.weather.texels[0] = TEXEL;
        profile.sun.size = 32;
        profile.sun.ground_size = 64;
        for level in &mut profile.sun.levels {
            level.slices = 16;
        }
        let sky = PlacedSky::from_state(store.into(), state.clone(), 1)
            .expect("placed sky")
            .with_profile(profile);
        Self {
            state,
            sky,
            cooker,
            source,
        }
    }

    pub(super) fn replace_stack(&mut self) -> u64 {
        let source = self.source.join("source/cloud-library.tvol");
        let mut bytes = fs::read(&source).expect("old stack");
        bytes[16 + 8 * 8 * 3 + 3] ^= 1;
        fs::write(source, bytes).expect("changed stack");
        self.publish_stacks(1)
    }

    pub(super) fn publish_stacks(&mut self, expected: usize) -> u64 {
        let report = self.cooker.cook_all().expect("cook stacks");
        assert_eq!(report.failed, []);
        assert_eq!(report.cooked, expected);
        report.published.expect("new publication").generation
    }

    pub(super) async fn host(&self, gpu: &Opened) -> Host {
        self.host_with_clouds(gpu, CloudSelection::Sky).await
    }

    pub(super) async fn host_with_clouds(&self, gpu: &Opened, clouds: CloudSelection) -> Host {
        let mut host = GameSetup {
            clouds,
            cloud_weather: self.state.parameters(),
            ..Default::default()
        }
        .prepare_placed(
            &gpu.device,
            &gpu.queue,
            &gpu.capabilities,
            frame_config(),
            self.sky.clone(),
        )
        .await
        .expect("prepare placed game");
        host.set_scene_camera(
            v1_game::SceneCamera::perspective(
                [0.0, 40.0, 0.0],
                [0.0, 1700.0, -1700.0],
                0.75,
                1.0,
                200_000.0,
            )
            .expect("camera"),
        )
        .expect("set camera");
        host
    }
}

pub(super) struct Frames<'a> {
    pub(super) gpu: &'a Opened,
    instant: Duration,
    view: wgpu::TextureView,
}

impl<'a> Frames<'a> {
    pub(super) fn new(gpu: &'a Opened) -> Self {
        Self {
            gpu,
            instant: Duration::ZERO,
            view: gpu
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("placed game pixels"),
                    size: wgpu::Extent3d {
                        width: EXTENT[0],
                        height: EXTENT[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }

    pub(super) fn submit(&mut self, host: &mut Host) {
        host.drive(
            self.instant,
            Some(&SurfaceOutput {
                view: &self.view,
                extent: EXTENT,
            }),
        )
        .expect("submit game frame");
        self.instant += Duration::from_millis(17);
        self.retire(host);
    }

    pub(super) fn retire(&self, host: &mut Host) {
        self.gpu
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal completion");
        while host.poll_completed().expect("poll completed").is_some() {}
    }

    pub(super) fn placed_submitted(
        &mut self,
        host: &mut Host,
        predicate: impl Fn(thrum_clouds::ShapesReport) -> bool,
    ) -> thrum_clouds::ShapesReport {
        thrum_test_support::wait::until("placed game submission", Duration::from_secs(4), || {
            self.submit(host);
            report(host).filter(|value| predicate(*value))
        })
    }
}

pub(super) fn report(host: &Host) -> Option<thrum_clouds::ShapesReport> {
    host.cloud_placed_input()?.shapes_report()
}

pub(super) fn frame_config() -> FrameConfig {
    FrameConfig {
        render_extent: EXTENT,
        surface: Some(thrum_render::present::SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent: EXTENT,
        }),
    }
}
