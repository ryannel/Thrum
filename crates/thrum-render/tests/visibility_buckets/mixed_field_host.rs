//! Host lifecycle witness using the shared mixed-mesh field.
use std::{sync::Arc, time::Duration};

use super::{Gpu, equal};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_fixtures::mixed_field::{self as field, day_cycle::DayCycle, meshes};
use thrum_render::{
    abi::MaterialRecord,
    atmosphere::AtmosphereFrame,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameError, FrameHost, FrameSample, FrameWorld, SceneFrame, SceneSource,
        SurfaceOutput,
    },
    synthetic::{
        CullMode, FrameLighting, SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes,
        SyntheticScene, SyntheticVertex, SyntheticView,
    },
};

struct Page {
    source: SourceTick,
    entities: Arc<[field::FieldEntity]>,
}
struct World(Page);
impl World {
    fn new(tick: u64) -> Self {
        let mut world = Self(Page {
            source: SourceTick {
                epoch: 47,
                timeline: 3,
                tick: 0,
            },
            entities: (0..field::ENTITY_COUNT).map(field::entity).collect(),
        });
        world.advance_to(tick);
        world
    }
    fn advance_to(&mut self, tick: u64) {
        assert!(tick >= self.0.source.tick);
        for _ in self.0.source.tick..tick {
            for entity in Arc::make_mut(&mut self.0.entities) {
                for axis in 0..3 {
                    entity.position[axis] += entity.velocity[axis] / 60.0;
                }
            }
        }
        self.0.source.tick = tick;
    }
}
impl FrameWorld for World {
    type Page = Page;
    fn source(&self) -> SourceTick {
        self.0.source
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _now: Duration) -> Result<WorldAdvance, FrameError> {
        Ok(WorldAdvance {
            tick: self.0.source.tick,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Page>, FrameError> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(self.0.source.epoch),
                tick: self.0.source.tick,
            },
            timeline: self.0.source.timeline,
            page: Page {
                source: self.0.source,
                entities: Arc::clone(&self.0.entities),
            },
        })
    }
    fn presentation_alpha_at(&self, _lead: Duration) -> f32 {
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
}
struct FieldSource;
impl SceneSource<Page> for FieldSource {
    fn extract(&self, into: &mut RenderExtractor, page: &Page) -> ExtractReport {
        into.extract(page.source, [0.0; 3], |sink| {
            for (index, entity) in page.entities.iter().enumerate() {
                let slot = u32::try_from(index).expect("field slot fits u32");
                assert!(sink.push(slot, 1, u128::from(slot), entity.position, entity.material));
            }
        })
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "witness extents are small integers"
    )]
    fn frame(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        let camera = field::camera(source.tick, alpha);
        let target = [
            camera.eye[0] + camera.heading.cos() * 10.0,
            camera.eye[1],
            camera.eye[2] + camera.heading.sin() * 10.0,
        ];
        let view = SyntheticView::perspective(
            camera.eye,
            target,
            [0.0, 1.0, 0.0],
            65.0_f32.to_radians(),
            extent[0] as f32 / extent[1] as f32,
            0.1,
            1200.0,
        )?
        .with_history_epoch(camera.history_epoch);
        let sun = DayCycle::sample(source.tick, alpha, 0.0);
        let atmosphere = AtmosphereFrame::new(
            sun.toward_sun,
            sun.top_illuminance_lux,
            sun.colour,
            camera.eye[1],
            sun.profile_epoch,
        )?
        .with_nightglow()
        .with_simulation_time(sun.simulation_time_seconds)?;
        let lighting = lighting.unwrap_or(FrameLighting::Atmosphere(atmosphere));
        let view = match lighting {
            FrameLighting::Atmosphere(_) => view
                .with_incident_exposure(0.0, -12.0, 18.0)
                .and_then(SyntheticView::with_low_light_adaptation),
            FrameLighting::Diagnostic(_) | FrameLighting::Photometric(_) => {
                view.with_exposure(1.0 / 16_000.0)
            }
        }?;
        Ok(SceneFrame { view, lighting })
    }
}
type Host = FrameHost<World, FieldSource>;
const RENDER: [u32; 2] = [320, 180];
fn profile(extent: [u32; 2]) -> SurfaceProfile {
    SurfaceProfile {
        format: wgpu::TextureFormat::Rgba8Unorm,
        extent,
    }
}

fn field_meshes() -> Arc<SyntheticMeshes> {
    Arc::new(SyntheticMeshes {
        updates: None,
        meshes: meshes::prop_meshes()
            .into_iter()
            .map(|mesh| SyntheticMesh {
                vertices: mesh
                    .vertices
                    .into_iter()
                    .map(|vertex| SyntheticVertex {
                        position: vertex.position,
                        normal: vertex.normal,
                        uv: vertex.uv,
                        tangent: vertex.tangent,
                    })
                    .collect(),
                indices: mesh.indices,
            })
            .collect(),
        slots: (0..field::ENTITY_COUNT)
            .map(|index| {
                meshes::mesh_slot(index, true)
                    .map_or(SyntheticMeshChoice::Cube, SyntheticMeshChoice::Mesh)
            })
            .collect(),
    })
}

async fn host(gpu: &Gpu, mode: CullMode, surface: [u32; 2], tick: u64) -> Host {
    let mut scene = SyntheticScene::grid(u32::try_from(field::ENTITY_COUNT).expect("field size"))
        .expect("field scene");
    scene.materials = field::materials()
        .into_iter()
        .map(|material| MaterialRecord {
            base_rgba: material.base_rgba,
            surface: material.surface,
            ..Default::default()
        })
        .collect();
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        let entity = field::entity(index);
        let (mut basis, mut normal) = field::basis(&entity);
        if meshes::mirrored(index) {
            basis[0] = basis[0].map(|v| -v);
            normal[0] = normal[0].map(|v| -v);
        }
        for axis in 0..3 {
            instance.current[axis][..3].copy_from_slice(&basis[axis]);
            instance.normal[axis][..3].copy_from_slice(&normal[axis]);
        }
        instance.previous = instance.current;
    }
    let meshes = field_meshes();
    assert_eq!(meshes.meshes.len(), 3, "rock, column and spruce");
    assert_eq!(meshes.slots.len(), field::ENTITY_COUNT);
    scene.meshes = Some(meshes);
    scene.sun_shadows = true;
    scene.camera_culling = mode;
    FrameHost::new(
        &gpu.caps,
        &gpu.queue,
        FrameConfig {
            render_extent: RENDER,
            surface: Some(profile(surface)),
        },
        World::new(tick),
        FieldSource,
        scene,
        0.125,
    )
    .await
    .expect("mixed-field host")
}

struct Drawable {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    extent: [u32; 2],
}
impl Drawable {
    fn new(gpu: &Gpu, extent: [u32; 2]) -> Self {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("host lifecycle drawable"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: profile(extent).format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            extent,
        }
    }
    fn pixels(&self, gpu: &Gpu) -> Vec<u8> {
        thrum_test_support::readback::texture(
            &gpu.device,
            &gpu.queue,
            &self.texture,
            thrum_test_support::readback::Mip::level(0),
            ..,
        )
    }
}
fn frame(gpu: &Gpu, host: &mut Host, drawable: &Drawable, now: Duration) -> (FrameSample, Vec<u8>) {
    let report = host
        .drive(
            now,
            Some(&SurfaceOutput {
                view: &drawable.view,
                extent: drawable.extent,
            }),
        )
        .expect("host drive");
    assert!(
        report.submitted && report.surface_presented,
        "real presenter submits the drawable"
    );
    // Each attempt waits up to 20 s for the queue; the old loop had no overall bound.
    let sample = thrum_test_support::wait::until(
        "host frame completion",
        thrum_test_support::readback::WAIT,
        || {
            gpu.wait();
            host.poll_completed().expect("completion")
        },
    );
    assert!(sample.surface_presented);
    assert_eq!(sample.tick, host.world().source().tick);
    (sample, drawable.pixels(gpu))
}
fn retain(label: &str, mode: CullMode, pixels: &[u8], extent: [u32; 2]) {
    if let Some(path) = std::env::var_os("THRUM_CULLING_LIFECYCLE_EVIDENCE") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).expect("lifecycle evidence directory");
        thrum_test_support::ppm::write(
            path.join(format!("{label}-{mode:?}.ppm")),
            extent,
            pixels,
            thrum_test_support::ppm::Pixels::Rgba8,
        )
        .expect("lifecycle surface pixels");
    }
}
#[expect(
    clippy::print_stderr,
    reason = "named-target lifecycle evidence records every frame's counts"
)]
fn phase(
    gpu: &Gpu,
    oracle: &mut Host,
    hosts: &mut [Host],
    extent: [u32; 2],
    clock: &mut u64,
    label: &str,
) -> Vec<u8> {
    let drawables: Vec<_> = (0..3).map(|_| Drawable::new(gpu, extent)).collect();
    let mut hidden = 0;
    let mut final_direct = Vec::new();
    for frame_index in 0..4 {
        *clock += 1;
        let now = Duration::from_millis(*clock * 16);
        assert_eq!(oracle.camera_culling(), CullMode::Direct);
        let (_, direct) = frame(gpu, oracle, &drawables[0], now);
        assert_eq!(direct.len(), extent[0] as usize * extent[1] as usize * 4);
        assert!(
            direct
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3] != direct[..3]),
            "field surface is not a blank image"
        );
        let lower_half = direct
            .as_chunks::<4>()
            .0
            .iter()
            .skip(extent[0] as usize * (extent[1] / 2) as usize);
        let lit_geometry = lower_half.filter(|pixel| pixel[..3] != [0, 0, 0]).count();
        assert!(
            lit_geometry > extent[0] as usize * extent[1] as usize / 10,
            "daylit field geometry must be visible, not a black oracle",
        );
        let mut counts = Vec::new();
        for ((host, drawable), mode) in hosts
            .iter_mut()
            .zip(&drawables[1..])
            .zip([CullMode::Frustum, CullMode::Occlusion])
        {
            assert_eq!(host.camera_culling(), mode, "resolved mode {label}");
            assert_eq!(
                host.world().source(),
                oracle.world().source(),
                "retained source {label}"
            );
            let (sample, pixels) = frame(gpu, host, drawable, now);
            equal(
                &direct,
                &pixels,
                &format!("host {label} frame {frame_index} {mode:?}"),
            );
            let count = sample
                .camera_culling
                .expect("drained submitted frame has counts");
            assert_eq!(count.mode, mode);
            assert_eq!(count.active as usize, field::ENTITY_COUNT);
            counts.push(count.drawn);
            if frame_index == 0 || frame_index == 3 {
                retain(&format!("{label}-{frame_index}"), mode, &pixels, extent);
            }
        }
        assert!(counts[1] <= counts[0]);
        hidden += usize::from(counts[1] < counts[0]);
        eprintln!(
            "host {label} frame {frame_index}: device {}, source {:?}, surface {extent:?}, Frustum {}, Occlusion {}, equal to fresh mixed-field Direct",
            gpu.caps.generation(),
            oracle.world().source(),
            counts[0],
            counts[1]
        );
        if frame_index == 0 || frame_index == 3 {
            retain(
                &format!("{label}-{frame_index}"),
                CullMode::Direct,
                &direct,
                extent,
            );
        }
        final_direct = direct;
    }
    assert!(
        hidden >= 2,
        "{label}: Occlusion renews hiding after its history seed"
    );
    final_direct
}

#[test]
fn mixed_field_host_surface_resize_and_device_publication_keep_occlusion_hidden() {
    let Some(gpu) = thrum_test_support::gpu(
        "mixed_field_host_surface_resize_and_device_publication_keep_occlusion_hidden",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let first = Gpu::new(&gpu, 71);
        let second = Gpu::new(&gpu.fresh_adapter(), 72);
        assert!(
            !second.caps.matches(&first.device),
            "distinct replacement device"
        );
        assert!(second.caps.generation() > first.caps.generation());
        let mut hosts = Vec::new();
        for mode in [CullMode::Frustum, CullMode::Occlusion] {
            hosts.push(host(&first, mode, RENDER, 1).await);
        }
        let mut oracle = host(&first, CullMode::Direct, RENDER, 1).await;
        let mut clock = 0;
        phase(
            &first,
            &mut oracle,
            &mut hosts,
            RENDER,
            &mut clock,
            "before",
        );
        let resized = [481, 271];
        for host in &mut hosts {
            host.resize_surface(resized)
                .expect("running host surface resize");
        }
        // A fresh oracle avoids sharing a faulty resize path with the recovered hosts.
        oracle = host(&first, CullMode::Direct, resized, 1).await;
        let before_replacement = phase(
            &first,
            &mut oracle,
            &mut hosts,
            resized,
            &mut clock,
            "resized",
        );
        let mut candidates = Vec::new();
        for host in &hosts {
            candidates.push(
                host.device_prepare_inputs()
                    .expect("retained mixed-field inputs")
                    .prepare(&second.caps, &second.queue, Some(profile(resized)))
                    .await
                    .expect("different-device preparation"),
            );
        }
        for (host, candidate) in hosts.iter_mut().zip(&mut candidates) {
            // Authority advances after detached preparation and before publication.
            host.world_mut().advance_to(61);
            host.publish_device(candidate)
                .expect("publish on the existing host");
            assert_eq!(
                host.world().source().tick,
                61,
                "publication retains current authority"
            );
        }
        // This oracle is cold-prepared from explicit authored meshes/current state,
        // so losing mesh selection in both replacement paths cannot make equality pass.
        oracle = host(&second, CullMode::Direct, resized, 61).await;
        let replaced = phase(
            &second,
            &mut oracle,
            &mut hosts,
            resized,
            &mut clock,
            "replaced",
        );
        assert_ne!(
            replaced, before_replacement,
            "the changed current scene is visible"
        );
    });
}
