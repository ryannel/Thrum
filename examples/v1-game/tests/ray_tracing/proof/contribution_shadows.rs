//! Cooked receivers, cooked wall casters and map-only secondary views.

use super::casters::World;
use super::*;
use glam::{Mat4, Vec3};
use thrum_render::{
    contribution::{
        ContributionCandidate, ContributionFrame, ContributionPasses, SceneContribution,
        SceneContributions, SceneSurface, SecondaryView, SecondaryViewRequest, SecondaryViewSlot,
        ViewAir,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameError, FrameHost as EngineHost, SceneFrame, SceneSource},
    synthetic::{
        FrameLighting, SunShadowMethod, SyntheticLight, SyntheticPointLight, SyntheticScene,
        SyntheticView,
    },
};

mod replacement;

pub(crate) fn prove_contributions() {
    let Some(metal) = Metal::new(1) else {
        return;
    };
    let mut fixture = Fixture::new("cooked-shadows");
    demo::write_grid_gltf(&fixture.cook.source, "receiver", 3, 0.0).expect("write cooked floor");
    fixture.publish();
    let mut secondary_maps = None;
    let mut lamp_clear = None;
    for (label, sun, method, lamps, wall) in [
        ("maps", true, SunShadowMethod::Maps, false, true),
        ("traced Sun", true, SunShadowMethod::Traced, false, true),
        ("lamp control", false, SunShadowMethod::Maps, false, true),
        ("traced lamps", false, SunShadowMethod::Maps, true, true),
        (
            "traced lamps without wall",
            false,
            SunShadowMethod::Maps,
            true,
            false,
        ),
    ] {
        let eye = [3.1, 2.2, 4.0];
        let aimed = |height| {
            SyntheticView::perspective(
                [eye[0], height, eye[2]],
                [0.0; 3],
                [0.0, 1.0, 0.0],
                0.8,
                EXTENT[0] as f32 / EXTENT[1] as f32,
                0.05,
                32.0,
            )
            .expect("fixture camera")
        };
        let view = aimed(eye[1]);
        let lighting = FrameLighting::Diagnostic(SyntheticLight {
            direction_to_light: [0.0, 0.55, -1.0],
            irradiance: [if sun { 6.0 } else { 0.0 }; 3],
        });
        let mut scene = SyntheticScene::grid(1).expect("fixture");
        scene.view = view;
        scene.instances[0].deactivate();
        scene.sun_shadows = sun;
        scene.sun_shadow_method = method;
        scene.lamp_shadows = lamps;
        if !sun {
            scene.local_lights = vec![SyntheticPointLight {
                position: [0.0, 1.1, -2.0],
                radius: 5.0,
                candela: [950.0; 3],
            }];
        }
        let mut host = pollster::block_on(EngineHost::new(
            &metal.caps,
            &metal.queue,
            frame_config(),
            World,
            Camera(view, lighting),
            scene,
            1.0,
        ))
        .expect("fixture host");
        let mut floor = MeshPlacement::new("receiver", "colour");
        floor.transform = Mat4::from_scale(Vec3::splat(2.5)).to_cols_array_2d();
        let mut placements = vec![floor];
        if wall {
            let mut wall = MeshPlacement::new("placement", "colour");
            wall.transform = (Mat4::from_translation(Vec3::new(1.84, 0.55, -1.0))
                * Mat4::from_scale(Vec3::splat(4.0)))
            .to_cols_array_2d();
            placements.push(wall);
        }
        let cooked = CookedAssets::new(CookedConfig::new(fixture.cook.store.clone(), placements))
            .expect("cooked shadow scene");
        let stats = cooked.stats();
        let observed = Arc::new(Mutex::new(None));
        let view_observer = ObserveView {
            slot: None,
            observed: Arc::clone(&observed),
            request: SecondaryViewRequest::new(&aimed(4.0), EXTENT, None, ViewAir::Camera)
                .expect("secondary view")
                .with_shadows(2, 16.0)
                .expect("own cascades"),
            traced: method == SunShadowMethod::Traced,
        };
        let contribution: Box<dyn SceneContribution> = if sun {
            Box::new(SceneContributions::new(vec![
                Box::new(view_observer),
                Box::new(cooked),
            ]))
        } else {
            Box::new(cooked)
        };
        host.set_scene_contribution(contribution)
            .expect("prepare contributions");
        let pixels = settle(&mut host, &stats, fixture.generation());
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked projected coordinates select pixels"
        )]
        let point = |world: Vec3| {
            let clip = Mat4::from_cols_array_2d(&view.view_proj_columns()) * world.extend(1.0);
            let x = ((0.5 + 0.5 * clip.x / clip.w) * EXTENT[0] as f32) as usize;
            let y = ((0.5 - 0.5 * clip.y / clip.w) * EXTENT[1] as f32) as usize;
            assert!(x < EXTENT[0] as usize && y < EXTENT[1] as usize);
            pixels[(y * EXTENT[0] as usize + x) * 4 + 1]
        };
        let dark = point(Vec3::ZERO);
        let clear = point(Vec3::new(-2.2, 0.0, 0.0));
        let mut secondary_digest = None;
        if sun {
            let texture = observed
                .lock()
                .expect("secondary observation")
                .clone()
                .expect("secondary colour");
            let bytes = copy_colour(&metal, &texture);
            assert!(
                bytes
                    .as_chunks::<8>()
                    .0
                    .windows(2)
                    .any(|pair| pair[0] != pair[1])
            );
            secondary_digest = Some(digest(&bytes));
            if let Some(maps) = &secondary_maps {
                assert!(
                    &bytes == maps,
                    "all secondary scene-linear bytes match Maps"
                );
                assert!(
                    dark <= 2 && clear > 20,
                    "cooked surface must take traced Sun: {dark}, {clear}"
                );
            } else {
                secondary_maps = Some(bytes);
            }
        } else if lamps && wall {
            assert!(
                lamp_clear.expect("lamp control") > 20 && dark <= 2 && clear > 20,
                "cooked wall must block lamp: {dark}, {clear}"
            );
        } else {
            assert!(
                dark > 20,
                "lamp has a clear shot at the cooked floor: {dark}"
            );
            lamp_clear = Some(dark);
        }
        report(
            &json!({"case":label, "under_wall_green":dark,"beside_wall_green":clear,
            "secondary_scene_linear_digest":secondary_digest, "sun_shadows":sun,"lamp_shadows":lamps}),
        );
    }
    replacement::prove(&metal, &mut fixture);
}

fn copy_colour(metal: &Metal, texture: &wgpu::Texture) -> Vec<u8> {
    let bytes_per_row = texture.width() * 8;
    let buffer = metal.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("secondary scene-linear readback"),
        size: u64::from(bytes_per_row) * u64::from(texture.height()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = metal
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    metal.queue.submit([encoder.finish()]);
    read(&metal.device, &buffer)
}

struct ObserveView {
    slot: Option<SecondaryView>,
    observed: Arc<Mutex<Option<wgpu::Texture>>>,
    request: SecondaryViewRequest,
    traced: bool,
}
impl SceneContribution for ObserveView {
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        vec![SecondaryViewSlot::new(1.0).expect("slot")]
    }
    fn secondary_views_enabled(&self) -> bool {
        true
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn supports_candidate_preparation(&self) -> bool {
        true
    }
    fn prepare_candidate(
        &self,
        surface: &SceneSurface<'_>,
    ) -> Result<ContributionCandidate, FrameError> {
        let slot = self.prepared_slot(surface);
        Ok(ContributionCandidate::new(
            self.passes(),
            self.effect_graph(),
            move |owner: &mut Self| {
                owner.accept_slot(slot);
            },
        ))
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.accept_slot(self.prepared_slot(surface));
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        self.slot
            .as_ref()
            .expect("slot")
            .request(&self.request)
            .expect("secondary recording");
    }
}

impl ObserveView {
    fn prepared_slot(&self, surface: &SceneSurface<'_>) -> SecondaryView {
        assert_eq!(
            surface.shader_source.contains("enable wgpu_ray_query"),
            self.traced
        );
        assert!(
            !surface.secondary_surfaces[0]
                .shader_source
                .contains("enable wgpu_ray_query")
        );
        surface.secondary_views[0].clone()
    }

    fn accept_slot(&mut self, slot: SecondaryView) {
        *self.observed.lock().expect("secondary observation") =
            Some(slot.colour().texture().clone());
        self.slot = Some(slot);
    }
}

struct Camera(SyntheticView, FrameLighting);
impl SceneSource<SourceTick> for Camera {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |_| {})
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.0,
            lighting: self.1,
        })
    }
}
