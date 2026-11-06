//! V1 fixture, camera and lighting policy for the engine frame runtime.

use crate::{environment::DayCycle, scene, simulation::PublishedPage};
#[cfg(not(target_arch = "wasm32"))]
use thrum_content_runtime::EnvironmentProduct;
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::synthetic::PhotometricEnvironment;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{SceneFrame, SceneSource},
    synthetic::{FrameLighting, SdrToneMap, SyntheticPointLight, SyntheticScene, SyntheticView},
};

fn atmosphere_lighting(tick: u64, alpha: f32, phase_offset: f32) -> FrameLighting {
    let sun = DayCycle::sample(tick, alpha, phase_offset);
    let frame = AtmosphereFrame::new(
        sun.toward_sun,
        sun.top_illuminance_lux,
        sun.colour,
        sun.camera_altitude_m,
        sun.profile_epoch,
    )
    .expect("fixed game day cycle produces a valid atmosphere frame");
    FrameLighting::Atmosphere(frame)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn photometric(product: &EnvironmentProduct) -> PhotometricEnvironment {
    let values = product.values();
    PhotometricEnvironment {
        toward_sun: values.toward_sun(),
        sun_lux: values.sun_lux(),
        sun_colour: values.sun_colour(),
        ambient_lux: values.ambient_lux(),
        background_radiance: values.background_radiance(),
    }
}

fn frame_phase(tick: u64, alpha: f32) -> f32 {
    if tick == 0 {
        return 0.0;
    }
    let cycle = u16::try_from(tick.saturating_sub(1) % 360).expect("phase is less than 360");
    (f32::from(cycle) + alpha) * (std::f32::consts::TAU / 360.0)
}

fn frame_view(
    tick: u64,
    alpha: f32,
    extent: [u32; 2],
    camera_pitch: f32,
    camera_yaw: f32,
) -> Result<SyntheticView, String> {
    let phase = frame_phase(tick, alpha);
    SyntheticView::perspective(
        [phase.sin() * 0.08, (phase.cos() - 1.0) * 0.04, 2.0],
        [
            2.0 * camera_yaw.sin(),
            2.0 * camera_pitch.tan(),
            2.0 - 2.0 * camera_yaw.cos(),
        ],
        [0.0, 1.0, 0.0],
        1.0,
        extent[0] as f32 / extent[1] as f32,
        0.1,
        20.0,
    )
    .map(|view| view.with_sdr_tone_map(SdrToneMap::NeutralHighlights))
    .map_err(|error| format!("live camera: {error:?}"))
}

pub(super) struct GameScene {
    pub(super) day_phase_offset: f32,
    pub(super) camera_pitch: f32,
    pub(super) camera_yaw: f32,
}
impl GameScene {
    pub(super) fn new() -> Self {
        Self {
            day_phase_offset: 0.0,
            camera_pitch: 0.0,
            camera_yaw: 0.0,
        }
    }
    pub(super) fn scene(extent: [u32; 2]) -> Result<SyntheticScene, String> {
        let count = u32::try_from(scene::ENTITY_COUNT).map_err(|_| "fixture exceeds u32 slots")?;
        let mut scene =
            SyntheticScene::grid(count).map_err(|error| format!("synthetic scene: {error:?}"))?;
        scene.view = frame_view(0, 0.0, extent, 0.0, 0.0)?;
        scene.sun_shadows = true;
        scene.local_lights = [
            ([-0.55, -0.55, 1.0], [5.0, 0.8, 0.4]),
            ([0.55, -0.55, 1.0], [0.4, 4.0, 0.8]),
            ([-0.55, 0.55, 1.0], [0.7, 1.0, 5.0]),
            ([0.55, 0.55, 1.0], [4.0, 3.0, 0.4]),
        ]
        .map(|(position, candela)| SyntheticPointLight {
            position,
            radius: 0.6,
            candela,
        })
        .to_vec();
        Ok(scene)
    }
}
impl SceneSource<PublishedPage> for GameScene {
    fn extract(&self, into: &mut RenderExtractor, page: &PublishedPage) -> ExtractReport {
        scene::extract(into, page)
    }
    fn frame(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        let lighting = lighting
            .unwrap_or_else(|| atmosphere_lighting(source.tick, alpha, self.day_phase_offset));
        let view = frame_view(
            source.tick,
            alpha,
            extent,
            self.camera_pitch,
            self.camera_yaw,
        )?;
        let view = match lighting {
            FrameLighting::Atmosphere(_) => view.with_incident_exposure(0.0, 0.0, 18.0),
            FrameLighting::Diagnostic(_) | FrameLighting::Photometric(_) => {
                view.with_exposure(1.0 / 16_000.0)
            }
        }
        .map_err(|error| format!("view exposure: {error:?}"))?;
        Ok(SceneFrame { view, lighting })
    }
}
