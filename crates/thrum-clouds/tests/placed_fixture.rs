use std::{fs, sync::Arc, time::Duration};
use thrum_asset::Profile as AssetProfile;
use thrum_asset_gpu::{CookedAssets, CookedConfig, LeaseRequest, ProductLease};
use thrum_cloud_model::{
    Directed, Parameters, PlacedLibrary, PlacedSlab, Profile, Shape, ShapeSource, State, Type,
};
use thrum_clouds::{
    Shade, ShadeConfig, ShapesConfig, SkyProfile, SunLevel, SunProfile, WeatherInput,
    WeatherProfile,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, encode_volume};
use thrum_render::{
    contribution::SceneContributions,
    runtime::{FrameConfig, FrameHost, SurfaceOutput},
};
use thrum_test_support::{Opened, Scratch};

use crate::scene_fixture;

pub(super) const GROUND: u32 = 32;
const RENDER: [u32; 2] = [66, 66];
pub(super) const TRACE_FOV: f32 = 1.76;
const SIDE: usize = 8;
const SPACING: f64 = 20.0;

pub(super) struct Fixture {
    store: Scratch,
    pub(super) config: ShadeConfig,
    pub(super) shape: Shape,
    library: Arc<PlacedLibrary>,
    pub(super) state: Arc<State>,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let store = Scratch::new("placed-density");
        let source = store.join("source");
        fs::create_dir(&source).expect("source folder");
        // One leaning body. Successive X slices change both density and height.
        // At the centre, the high eastern lobe leaves a Sun ray through the low west clear.
        let voxels: Vec<u8> = (0..SIDE.pow(3))
            .map(|at| {
                let x = at % SIDE;
                let y = (at / SIDE) % SIDE;
                let z = at / SIDE.pow(2);
                if !(2..=5).contains(&z) || !(1..=6).contains(&x) {
                    return 0;
                }
                if y == 1 + x / 2 {
                    return 225;
                }
                if y == 3 + x / 2 {
                    return 120;
                }
                if y == 2 + x / 2 && x.is_multiple_of(2) {
                    return 65;
                }
                0
            })
            .collect();
        fs::write(source.join("stack.tvol"), encode_volume(8, 8, 8, &voxels)).expect("stack");
        let mut cooker = Cooker::open(CookConfig {
            source,
            store: store.join("store"),
            profiles: vec![AssetProfile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("cooker");
        assert_eq!(cooker.cook_all().expect("cook fixture").failed, []);
        let shape = Shape::new(
            [SIDE; 3],
            SPACING,
            voxels.iter().map(|&v| f64::from(v) / 255.0).collect(),
        )
        .expect("shape");
        let footprint = Arc::new(shape.footprint().clone());
        let library = Arc::new(
            PlacedLibrary::new(
                vec![PlacedSlab {
                    extent: 160.0,
                    floor: 0.0,
                    heading: 0.0,
                    footprint: Arc::clone(&footprint),
                }],
                Vec::new(),
            )
            .expect("fixture library"),
        );
        let mut state = State::new(
            Parameters {
                coverage: 0.0,
                kind: 1.0,
                wind: [0.0; 2],
                time_scale: 1.0,
            },
            0.5,
            16,
        )
        .expect("weather");
        state.set_library(Arc::clone(&library)).expect("library");
        state
            .add_from_library(
                Directed {
                    id: 42,
                    centre: [20.0, 0.0],
                    yaw: 0.0,
                    scale: 1.0,
                    footprint,
                    envelope: 0.0,
                    target: 1.0,
                    growth: 0.0,
                    growth_target: 0.0,
                },
                ShapeSource::Library(0),
            )
            .expect("direct shape");
        state.step(1, 1.0).expect("form");
        let config = weather_config();
        Self {
            store,
            config,
            shape,
            library,
            state: Arc::new(state),
        }
    }

    pub(super) fn assets(&self) -> CookedAssets {
        let mut config = CookedConfig::new(self.store.join("store"), Vec::new());
        config.leased.push(LeaseRequest {
            name: "stack".into(),
            dimension: wgpu::TextureDimension::D3,
        });
        CookedAssets::new(config).expect("placed test assets")
    }

    pub(super) fn shade(&self, lease: ProductLease, range: [f64; 2], sky: bool) -> Shade {
        let shade = Shade::new(self.config.clone())
            .expect("weather config")
            .with_shapes(ShapesConfig {
                library: Arc::clone(&self.library),
                lease,
                range,
                capacity: 1,
            })
            .expect("placed config");
        if sky {
            shade
                .with_sky(SkyProfile {
                    extent: RENDER,
                    step: 5.0,
                    light_samples: 0,
                    refresh_stride: 1,
                })
                .expect("trace")
        } else {
            shade
        }
    }

    pub(super) async fn submit(
        &self,
        gpu: &Opened,
        range: [f64; 2],
        sky: bool,
        resident: bool,
        sun: [f32; 3],
    ) -> (
        FrameHost<scene_fixture::World, scene_fixture::Source>,
        WeatherInput,
    ) {
        self.submit_at(gpu, range, sky, resident, sun, 0.0).await
    }

    pub(super) async fn submit_at(
        &self,
        gpu: &Opened,
        range: [f64; 2],
        sky: bool,
        resident: bool,
        sun: [f32; 3],
        z: f32,
    ) -> (
        FrameHost<scene_fixture::World, scene_fixture::Source>,
        WeatherInput,
    ) {
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let view = column_view(z);
        let mut host = scene_fixture::prepare_view(
            &gpu.capabilities,
            &gpu.queue,
            Some(GROUND),
            FrameConfig {
                render_extent: RENDER,
                surface: Some(thrum_render::present::SurfaceProfile {
                    format,
                    extent: RENDER,
                }),
            },
            sun,
            1,
            &view,
        )
        .await;
        let assets = self.assets();
        let lease = if resident {
            assets.lease()
        } else {
            ProductLease::default()
        };
        let shade = self.shade(lease.clone(), range, sky);
        let input = shade.input();
        input
            .publish(Arc::clone(&self.state))
            .expect("publish fixture state");
        let mut members: Vec<Box<dyn thrum_render::contribution::SceneContribution>> = Vec::new();
        if resident {
            members.push(Box::new(assets));
        }
        members.push(Box::new(shade));
        host.set_scene_contribution(Box::new(SceneContributions::new(members)))
            .expect("attach shapes");
        let output = output(gpu);
        let mut now = Duration::ZERO;
        thrum_test_support::wait::until("lease load", Duration::from_secs(4), || {
            frame(gpu, &mut host, &output, now);
            if !resident || lease.offered().is_some() {
                return Some(());
            }
            now += Duration::from_millis(16);
            None
        });
        frame(gpu, &mut host, &output, now + Duration::from_millis(16));
        (host, input)
    }
}

pub(super) fn weather_config() -> ShadeConfig {
    let kind = Type {
        base: 80.0,
        depth: [80.0, 160.0],
        liquid_water: 0.0001,
        droplet_radius: 0.00001,
        deck: 0.0,
        profile: [0.2, 0.3],
    };
    ShadeConfig {
        seed: 7,
        model: Profile {
            pitch: 600.0,
            turret_pitch: 300.0,
            turret_least: 0.2,
            turret_fade: 0.2,
            period: 1800.0,
            detail_mean: 0.5,
            erosion: 0.0,
        },
        types: vec![kind; 2],
        weather: WeatherProfile {
            size: 16,
            texels: vec![10.0, 20.0, 40.0, 80.0],
            maximum_age: 0.5,
        },
        sun: SunProfile {
            size: 32,
            levels: [10.0, 30.0, 80.0].map(|texel| SunLevel {
                texel,
                slices: 32,
                maximum_age: 0.5,
            }),
            ground_size: GROUND,
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
    }
}

pub(super) fn column_view(z: f32) -> thrum_render::synthetic::SyntheticView {
    thrum_render::synthetic::SyntheticView::perspective(
        [0.0, 40.0, z],
        [0.0, 160.0, z],
        [0.0, 0.0, -1.0],
        TRACE_FOV,
        1.0,
        0.1,
        200_000.0,
    )
    .expect("vertical column view")
}

pub(super) fn output(gpu: &Opened) -> wgpu::TextureView {
    gpu.device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("placed fixture display"),
            size: wgpu::Extent3d {
                width: RENDER[0],
                height: RENDER[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(super) fn frame(
    gpu: &Opened,
    host: &mut FrameHost<scene_fixture::World, scene_fixture::Source>,
    output: &wgpu::TextureView,
    now: Duration,
) {
    host.drive(
        now,
        Some(&SurfaceOutput {
            view: output,
            extent: RENDER,
        }),
    )
    .expect("submit placed frame");
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("placed fixture complete");
    while host.poll_completed().expect("placed completion").is_some() {}
}

pub(super) fn ground(gpu: &Opened, input: &WeatherInput) -> Vec<u8> {
    let (texture, _) = input.ground_product().expect("ground field");
    thrum_test_support::readback::texture(
        &gpu.device,
        &gpu.queue,
        &texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    )
}
