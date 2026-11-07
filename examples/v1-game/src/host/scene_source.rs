//! V1 fixture, camera and lighting policy for the engine frame runtime.

use crate::{camera::SceneCamera, environment::DayCycle, scene, simulation::PublishedPage};
#[cfg(not(target_arch = "wasm32"))]
use thrum_content_runtime::EnvironmentProduct;
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::synthetic::PhotometricEnvironment;
use thrum_render::{
    abi::MaterialRecord,
    atmosphere::AtmosphereFrame,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{SceneFrame, SceneSource},
    synthetic::{FrameLighting, SdrToneMap, SyntheticPointLight, SyntheticScene, SyntheticView},
};

fn atmosphere_lighting(
    tick: u64,
    alpha: f32,
    phase_offset: f32,
    camera_altitude_m: f32,
) -> FrameLighting {
    let sun = DayCycle::sample(tick, alpha, phase_offset);
    let frame = AtmosphereFrame::new(
        sun.toward_sun,
        sun.top_illuminance_lux,
        sun.colour,
        camera_altitude_m,
        sun.profile_epoch,
    )
    .expect("checked game camera and day cycle produce a valid atmosphere frame");
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
    camera: Option<SceneCamera>,
) -> Result<SyntheticView, String> {
    if let Some(camera) = camera {
        return camera
            .view(extent, camera_pitch, camera_yaw)
            .map(|view| view.with_sdr_tone_map(SdrToneMap::NeutralHighlights));
    }
    let phase = frame_phase(tick, alpha);
    SyntheticView::perspective(
        [phase.sin() * 0.08, (phase.cos() - 1.0) * 0.04, 2.0],
        [
            2.0 * camera_yaw.sin(),
            2.0 * camera_pitch.tan(),
            2.0 - 2.0 * camera_yaw.cos(),
        ],
        [0.0, 1.0, 0.0],
        // Keep the initial edge rows and the neutral subject inside the view.
        65.0_f32.to_radians(),
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
    camera: Option<SceneCamera>,
}
impl GameScene {
    pub(super) fn new() -> Self {
        Self {
            day_phase_offset: 0.0,
            camera_pitch: 0.0,
            camera_yaw: 0.0,
            camera: None,
        }
    }
    pub(super) fn scene(extent: [u32; 2]) -> Result<SyntheticScene, String> {
        let count = u32::try_from(scene::ENTITY_COUNT).map_err(|_| "fixture exceeds u32 slots")?;
        let mut scene =
            SyntheticScene::grid(count).map_err(|error| format!("synthetic scene: {error:?}"))?;
        scene.materials[0].base_rgba = [0.18, 0.18, 0.18, 1.0];
        scene.materials[0].surface = [1.0, 1.0, 0.0, 1.0];
        scene.materials.push(MaterialRecord {
            base_rgba: [0.65, 0.65, 0.65, 1.0],
            surface: [1.0, 0.32, 1.0, 1.0],
            ..Default::default()
        });
        // Static authored subjects are outside the moving prefix and four
        // controlled slots. Their rotated faces expose lighting direction.
        // A 0.18 halfextent also stays outside the coloured local-light spheres.
        for slot in [scene::NEUTRAL_SUBJECT_SLOT, scene::REFLECTION_SUBJECT_SLOT] {
            let subject = &mut scene.instances[slot];
            let basis_length = subject.current[0][..3]
                .iter()
                .map(|value| value * value)
                .sum::<f32>()
                .sqrt();
            let scale = 0.18 / basis_length;
            for axis in 0..3 {
                for component in 0..3 {
                    subject.current[axis][component] *= scale;
                    subject.previous[axis][component] *= scale;
                    subject.normal[axis][component] /= scale;
                }
            }
        }
        // This cold geometry matches the authoritative spawn. Publication owns
        // position and material after extraction; no GPU identity override.
        let reflection = &mut scene.instances[scene::REFLECTION_SUBJECT_SLOT];
        reflection.current[3][..3].copy_from_slice(&scene::REFLECTION_SUBJECT_POSITION);
        reflection.previous[3][..3].copy_from_slice(&scene::REFLECTION_SUBJECT_POSITION);
        scene.view = frame_view(0, 0.0, extent, 0.0, 0.0, None)?;
        scene.sun_shadows = true;
        scene.local_lights = [
            ([-0.9, -0.9, 1.0], [5.0, 0.8, 0.4]),
            ([0.9, -0.9, 1.0], [0.4, 4.0, 0.8]),
            ([-0.9, 0.9, 1.0], [0.7, 1.0, 5.0]),
            ([0.9, 0.9, 1.0], [4.0, 3.0, 0.4]),
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

impl super::FrameHost {
    /// Current presentation camera pitch and yaw, in radians.
    #[must_use]
    pub fn camera_orientation(&self) -> [f32; 2] {
        let source = self.inner.source();
        [source.camera_pitch, source.camera_yaw]
    }

    /// Select a checked authored camera and reset presentation history.
    pub fn set_scene_camera(&mut self, camera: SceneCamera) {
        let source = self.inner.source_mut();
        [source.camera_pitch, source.camera_yaw] = camera.orientation();
        source.camera = Some(camera);
        self.inner.invalidate_history();
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
        let view = frame_view(
            source.tick,
            alpha,
            extent,
            self.camera_pitch,
            self.camera_yaw,
            self.camera,
        )?;
        // The diagnostic fixture's local Y zero is a two-metre reference datum.
        // Authored scene coordinates already measure altitude above sea level.
        let camera_altitude_m =
            view.camera_position()[1] + if self.camera.is_some() { 0.0 } else { 2.0 };
        let lighting = lighting.unwrap_or_else(|| {
            atmosphere_lighting(source.tick, alpha, self.day_phase_offset, camera_altitude_m)
        });
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

#[cfg(test)]
mod tests {
    use super::{GameScene, RenderExtractor, SceneSource, SourceTick, frame_view, scene};
    use crate::{camera::SceneCamera, environment::DayCycle};
    use thrum_core::StableId;
    use thrum_render::synthetic::{FrameLighting, SdrToneMap, SyntheticView};

    #[test]
    fn diagnostic_camera_keeps_its_original_pose_and_bob() {
        let extent = [640, 360];
        let actual = frame_view(0, 0.0, extent, 0.0, 0.0, None).unwrap();
        let expected = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            65.0_f32.to_radians(),
            640.0 / 360.0,
            0.1,
            20.0,
        )
        .unwrap()
        .with_sdr_tone_map(SdrToneMap::NeutralHighlights);
        assert_eq!(actual, expected);
        let bobbed = frame_view(91, 1.0, extent, 0.0, 0.0, None).unwrap();
        assert_ne!(actual.camera_position(), bobbed.camera_position());
        assert!((1.92..=2.0).contains(&(2.0 + bobbed.camera_position()[1])));
    }

    #[test]
    fn authored_camera_is_fixed_across_ticks_and_preserves_the_world_sun() {
        // The supported atmosphere ceiling exercises the same view's altitude:
        // applying the legacy two-metre datum would exceed the profile bounds.
        let camera = SceneCamera::perspective(
            [0.0, 100_000.0, 16.0],
            [0.0, 100_000.0, -60.0],
            46.0_f32.to_radians(),
            0.1,
            30_000.0,
        )
        .unwrap();
        let [camera_pitch, camera_yaw] = camera.orientation();
        let policy = GameScene {
            day_phase_offset: 0.1,
            camera_pitch,
            camera_yaw,
            camera: Some(camera),
        };
        let first = policy
            .frame(
                SourceTick {
                    epoch: 1,
                    tick: 1,
                    timeline: 0,
                },
                0.5,
                [640, 360],
                None,
            )
            .unwrap();
        for (tick, extent) in [(91, [640, 360]), (181, [360, 640])] {
            let frame = policy
                .frame(
                    SourceTick {
                        epoch: 1,
                        tick,
                        timeline: 0,
                    },
                    0.5,
                    extent,
                    None,
                )
                .unwrap();
            assert_eq!(first.view.camera_position(), frame.view.camera_position());
            let FrameLighting::Atmosphere(atmosphere) = frame.lighting else {
                panic!("authored camera retains atmosphere lighting");
            };
            let expected = DayCycle::sample(tick, 0.5, 0.1).toward_sun;
            assert!(
                atmosphere
                    .toward_sun()
                    .into_iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 1e-6)
            );
            if extent == [640, 360] {
                assert_eq!(first.view, frame.view);
            } else {
                assert_ne!(
                    first.view.view_proj_columns(),
                    frame.view.view_proj_columns()
                );
            }
        }
    }

    #[test]
    fn reflection_subject_keeps_authoritative_material_and_static_history() {
        let mut world = scene::simulation().expect("actual 10k fixture");
        let mut render = GameScene::scene([640, 360]).expect("authored lighting subjects");
        let mut extractor = RenderExtractor::new(scene::ENTITY_COUNT, scene::ENTITY_COUNT);
        let handle = world
            .handle(StableId(scene::REFLECTION_SUBJECT_SLOT as u128))
            .expect("authored subject exists");
        let slot = usize::try_from(handle.slot).expect("bounded physical slot");
        let policy = GameScene::new();
        let mut changed = Vec::new();
        for _ in 0..2 {
            let page = world.published_page().expect("committed game page");
            assert_eq!(
                page.visual(handle),
                Some(scene::REFLECTION_SUBJECT_MATERIAL)
            );
            policy.extract(&mut extractor, &page);
            let report = render.apply_render_columns(extractor.frame(), &mut changed);
            assert_eq!(report.dropped_invalid, 0);
            let subject = &render.instances[slot];
            assert_eq!(
                u64::from(subject.ids[1]),
                scene::REFLECTION_SUBJECT_MATERIAL
            );
            assert_eq!(subject.current, subject.previous);
            for (basis, normal) in subject.current[..3].iter().zip(&subject.normal) {
                let inverse_product: f32 = basis[..3]
                    .iter()
                    .zip(&normal[..3])
                    .map(|(a, b)| a * b)
                    .sum();
                assert!((inverse_product - 1.0).abs() < 1e-6);
            }
            for index in [scene::NEUTRAL_SUBJECT_SLOT, slot] {
                let subject = &render.instances[index];
                let radius = subject.current[..3]
                    .iter()
                    .flat_map(|axis| &axis[..3])
                    .map(|value| value * value)
                    .sum::<f32>()
                    .sqrt();
                for light in &render.local_lights {
                    let distance = subject.current[3][..3]
                        .iter()
                        .zip(light.position)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f32>()
                        .sqrt();
                    assert!(
                        distance > radius + light.radius,
                        "lighting subject intersects a local-light sphere"
                    );
                }
            }
            drop(page);
            world
                .step(&[])
                .expect("same authoritative workload advances");
        }
    }
}
