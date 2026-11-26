//! Small public frame host with one authored row and a dry surface.
#![expect(
    dead_code,
    reason = "the medium and band binaries exercise different arms"
)]

use crate::scene_fixture::{Source, World};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_cloud_model::{
    Affinity, Field, Fog, Ground, GroundProfile, GroundState, Parameters, Profile, State, Surface,
    SurfacePoint, Type,
};
use thrum_clouds::{
    GroundConfig, GroundProducts, Shade, ShadeConfig, SunLevel, SunProfile, WeatherProfile,
};
use thrum_render::{
    atmosphere::{AtmosphereFrame, EnvironmentProducts, SunOpticalDepth},
    capabilities::DeviceCapabilities,
    contribution::{
        ContributionFrame, ContributionPasses, SceneContribution, SceneContributions, SceneSurface,
    },
    runtime::{FrameConfig, FrameError, FrameHost, SurfaceOutput},
    synthetic::SyntheticView,
    synthetic::{FrameLighting, SyntheticScene},
};
use thrum_test_support::{Opened, readback};

pub(crate) const EXTENT: [u32; 2] = [32, 16];
pub(crate) const SEED: u32 = 52;
pub(crate) const PROFILE: Profile = Profile {
    pitch: 3_000.0,
    turret_pitch: 800.0,
    turret_least: 0.2,
    turret_fade: 0.12,
    period: 1800.0,
    detail_mean: 0.5,
    erosion: 0.0,
};
pub(crate) const KIND: Type = Type {
    base: 900.0,
    depth: [240.0; 2],
    liquid_water: 0.000_008,
    droplet_radius: 10e-6,
    deck: 1.0,
    profile: [0.1; 2],
};

pub(crate) fn open() -> Option<Opened> {
    thrum_test_support::open(
        "cloud ground provider",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
}

pub(crate) fn fog(drifting: bool) -> Fog {
    Fog {
        liquid_water: 0.000_016,
        droplet_radius: 10e-6,
        depth: 64.0,
        depression: 0.0,
        softness: 12.0,
        pooling: 0.0,
        drifting,
        affinity: Affinity::Any,
        centre: [160.0, 0.0],
        radius: [85.0, 100.0],
        noise: 0.0,
        noise_scale: 30.0,
        radiative: false,
    }
}

pub(crate) fn fixture(row: Fog, ridge: bool) -> GroundConfig {
    let nodes: Vec<_> = (0..3)
        .flat_map(|_| {
            (0..17).map(|x| SurfacePoint {
                ground: if ridge && x == 12 { 60.0 } else { 0.0 },
                water: None,
            })
        })
        .collect();
    let surface = Surface::new([-320.0, -80.0], [17, 3], 40.0, 3000.0, &nodes).expect("surface");
    let fog = Ground::new(
        GroundProfile {
            shore_margin: 40.0,
            calm: [2.0, 8.0],
            twilight: [-6.0, 10.0],
            cooling_seconds: 10_800.0,
            warming_seconds: 3600.0,
        },
        vec![row],
    )
    .expect("fog table");
    GroundConfig {
        surface: Arc::new(surface),
        fog: Arc::new(fog),
    }
}

pub(crate) fn field() -> Field {
    Field::new(SEED, PROFILE, vec![KIND; 2]).expect("headless weather field")
}

pub(crate) fn state(coverage: f64, amount: f64) -> State {
    let mut state = State::new(
        Parameters {
            coverage,
            kind: 0.0,
            wind: [0.0; 2],
            time_scale: 1.0,
        },
        5.0,
        0,
    )
    .expect("fixture state");
    state.set_ground(GroundState::new(vec![amount], 0.0, 100_000.0).expect("fixture ground"));
    state
}

pub(crate) fn render(
    gpu: &Opened,
    ground: &GroundConfig,
    state: &State,
    elevation: f32,
) -> (
    FrameHost<World, Source>,
    GroundProducts,
    EnvironmentProducts,
) {
    render_from_height(gpu, ground, state, elevation, 12.0)
}

pub(crate) fn render_from_height(
    gpu: &Opened,
    ground: &GroundConfig,
    state: &State,
    elevation: f32,
    eye_y: f32,
) -> (
    FrameHost<World, Source>,
    GroundProducts,
    EnvironmentProducts,
) {
    let config = ShadeConfig {
        seed: SEED,
        model: PROFILE,
        types: vec![KIND; 2],
        weather: WeatherProfile {
            size: 16,
            texels: vec![20.0, 80.0, 320.0, 3200.0],
            maximum_age: 0.5,
        },
        sun: SunProfile {
            size: 16,
            levels: [80.0, 320.0, 1600.0].map(|texel| SunLevel {
                texel,
                slices: 16,
                maximum_age: 0.5,
            }),
            ground_size: 16,
            lowest_elevation: 4.0_f64.to_radians(),
            border_fraction: 0.1,
        },
    };
    let shade = Shade::new(config)
        .expect("weather")
        .with_ground(ground.clone());
    let input = shade.input();
    input
        .publish(Arc::new(state.clone()))
        .expect("ground state");
    let view = SyntheticView::perspective(
        [0.0, eye_y, 0.0],
        [1.0, eye_y, 0.0],
        [0.0, 1.0, 0.0],
        10.0_f32.to_radians(),
        2.0,
        0.1,
        4000.0,
    )
    .expect("fixture camera");
    let a = elevation.to_radians();
    let provider = shade.provider().expect("ground band target");
    let witness = Arc::new(Mutex::new(None));
    let contribution = SceneContributions::new(vec![
        Box::new(shade),
        Box::new(Receiver(Arc::clone(&witness))),
    ]);
    let mut host = pollster::block_on(prepare_ground(
        &gpu.capabilities,
        &gpu.queue,
        &view,
        [a.cos(), a.sin(), 0.0],
        Box::new(contribution),
        provider,
    ));
    let output = gpu
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("ground test output"),
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
        .create_view(&wgpu::TextureViewDescriptor::default());
    assert!(
        host.drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &output,
                extent: EXTENT
            })
        )
        .expect("submit ground frame")
        .submitted
    );
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("complete frame");
    while host.poll_completed().expect("retire frame").is_some() {}
    (
        host,
        input.ground_condensate_products().expect("ground products"),
        witness
            .lock()
            .expect("receiver witness")
            .take()
            .expect("receiver environment"),
    )
}

struct Receiver(Arc<Mutex<Option<EnvironmentProducts>>>);

impl SceneContribution for Receiver {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        *self.0.lock().expect("receiver witness") = surface.environment_products_with_cloud();
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

pub(crate) fn read(gpu: &Opened, texture: &wgpu::Texture, mip: readback::Mip) -> Vec<[f32; 4]> {
    let bytes = readback::texture(&gpu.device, &gpu.queue, texture, mip, ..);
    bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|bytes| {
            std::array::from_fn(|channel| {
                let bits = u16::from_le_bytes([bytes[2 * channel], bytes[2 * channel + 1]]);
                let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
                let exponent = i32::from(bits >> 10 & 31);
                let fraction = f32::from(bits & 1023);
                sign * if exponent == 0 {
                    fraction * 2.0_f32.powi(-24)
                } else {
                    (1.0 + fraction / 1024.0) * 2.0_f32.powi(exponent - 15)
                }
            })
        })
        .collect()
}

async fn prepare_ground(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    view: &SyntheticView,
    sun: [f32; 3],
    contribution: Box<dyn SceneContribution>,
    sun_provider: SunOpticalDepth,
) -> FrameHost<World, Source> {
    let mut scene = SyntheticScene::grid(1).expect("fixture scene");
    scene.view = *view;
    scene.sun_optical_depth = Some(sun_provider);
    let light = AtmosphereFrame::new(sun, 100_000.0, [1.0; 3], view.camera_position()[1], 1)
        .expect("physical light")
        .with_view_volume_medium(1);
    let extent = [32, 16];
    let mut host = Box::pin(FrameHost::new_preparing(
        caps,
        queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(thrum_render::present::SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent,
            }),
        },
        World::at_tick(0),
        Source::with_light(view, FrameLighting::Atmosphere(light)),
        scene,
        1.0,
    ))
    .await
    .expect("public frame host");
    host.attach_scene_contribution(contribution)
        .expect("prepare Ground");
    host.finish_preparation();
    host
}
