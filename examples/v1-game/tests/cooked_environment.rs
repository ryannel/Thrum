//! Opt-in accepted source -> cook -> prepared Metal view A/B integration.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use thrum_cook::cook_render_environment;
use thrum_source::{
    NativeSourceStore, SourcePath, SourceSpacePlan, registry_stage::validate_staged_candidate,
    stage_declared_tree_with_plan,
};
use v1_game::FrameHost;

const PROJECT: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const SCENE: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const ENVIRONMENT: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const LIGHT: &str = "5a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const HISTORY: &str = "6a5ef644-7225-4770-9f4d-e1d9c0a7d515";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "thrum-cooked-render-{}-{sequence}",
            std::process::id()
        ));
        for directory in [".engine", "scenes", "records", "schemas"] {
            fs::create_dir_all(root.join(directory)).expect("fixture or GPU operation");
        }
        Self(root)
    }

    fn plan() -> SourceSpacePlan {
        SourceSpacePlan {
            main_target_bytes: 1 << 30,
            wal_stop_bytes: 1 << 26,
            temporary_target_bytes: 1 << 26,
            free_headroom_bytes: 0,
        }
    }

    fn write(&self, colour: [f64; 3], sun_lux: u32) -> Vec<SourcePath> {
        let records = [
            "records/environment.record.json",
            "records/light.record.json",
            "scenes/main.scene.json",
        ];
        fs::write(
            self.0.join("project.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":PROJECT,"type":"core.project","schema":2,
                "fields":{"project.format":1,"project.records":records,
                    "project.schemas":[],"project.registryHistory":"schemas/history.registry.json",
                    "project.entryScenes":[SCENE]}
            }))
            .expect("fixture or GPU operation"),
        )
        .expect("fixture or GPU operation");
        fs::write(
            self.0.join("scenes/main.scene.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":SCENE,"type":"core.scene","schema":2,
                "fields":{"environment.source":ENVIRONMENT}
            }))
            .expect("fixture or GPU operation"),
        )
        .expect("fixture or GPU operation");
        fs::write(
            self.0.join("records/environment.record.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":ENVIRONMENT,"type":"thrum.environment.authored","schema":1,
                "fields":{"profile":"constant-light-v1",
                    "sun":{"light_ref":LIGHT,"toward_source_unit":[0,0,1],
                        "perpendicular_illuminance_lux":sun_lux,
                        "unit_luminance_linear_rec709":colour,
                        "angular_radius_rad":0.00465,
                        "direction_model":"geometric-straight","applied_attenuation":[]},
                    "ambient_irradiance_lux":[1000,1000,1000],
                    "background_radiance_cd_m2":[0,0,0],
                    "unsupported":["wind","cloud_tau","precipitation","air","thermal","moon","air_quality"]}
            }))
            .expect("fixture or GPU operation"),
        )
        .expect("fixture or GPU operation");
        fs::write(
            self.0.join("records/light.record.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":LIGHT,"type":"thrum.light.identity","schema":1,
                "fields":{"driver_binding":"incoming-environment-sun"}
            }))
            .expect("fixture or GPU operation"),
        )
        .expect("fixture or GPU operation");
        fs::write(
            self.0.join("schemas/history.registry.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":HISTORY,"type":"core.registry-history","schema":1,
                "fields":{"registry.fields":[],"registry.versions":[]}
            }))
            .expect("fixture or GPU operation"),
        )
        .expect("fixture or GPU operation");
        let mut paths = vec!["project.json", "schemas/history.registry.json"];
        paths.extend(records);
        paths.sort_unstable();
        paths
            .into_iter()
            .map(|path| SourcePath::parse(path).expect("fixture or GPU operation"))
            .collect()
    }

    fn admit(
        &self,
        store: &NativeSourceStore,
        old: i64,
        key: i64,
        paths: &[SourcePath],
    ) -> thrum_source::AcceptedSourceRoot {
        let stage = stage_declared_tree_with_plan(&self.0, paths, Self::plan())
            .expect("fixture or GPU operation");
        let candidate = validate_staged_candidate(&stage, &[]).expect("fixture or GPU operation");
        store
            .admit(
                old,
                1,
                key,
                [u8::try_from(key).expect("fixture or GPU operation"); 32],
                &candidate,
            )
            .expect("fixture or GPU operation")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("fixture or GPU operation");
    }
}

#[test]
#[ignore = "requires a real native GPU adapter for accepted A/B source pixel and tick proof"]
fn accepted_a_b_visuals_publish_without_restarting_the_world() {
    pollster::block_on(async {
        let fixture = Fixture::new();
        let store =
            NativeSourceStore::open(&fixture.0.join(".engine/source.sqlite"), Fixture::plan())
                .expect("fixture or GPU operation");
        let a_paths = fixture.write([1.0; 3], 100_000);
        let a = fixture.admit(&store, 0, 1, &a_paths);
        let mut pin = store
            .pin_current(a.generation)
            .expect("fixture or GPU operation");
        let product_a = cook_render_environment(&mut pin).expect("fixture or GPU operation");
        pin.close().expect("fixture or GPU operation");

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let timestamps = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features:
                    thrum_render::capabilities::DeviceCapabilities::requested_timing_features(
                        &adapter,
                    ),
                ..Default::default()
            })
            .await
            .expect("device");
        let mut host = FrameHost::new(&device, &queue, v1_game::FrameConfig::default())
            .await
            .expect("fixture or GPU operation");
        let mut candidate_a = host
            .visual_prepare_inputs()
            .prepare(product_a.runtime_product())
            .await
            .expect("fixture or GPU operation");
        assert!(
            host.publish_environment(&mut candidate_a)
                .expect("fixture or GPU operation")
        );
        let tick_a = host
            .drive(Duration::from_millis(17), None)
            .expect("fixture or GPU operation");
        assert_eq!(tick_a.completed_ticks, 1);
        let pixels_a = host.capture_pixels().expect("fixture or GPU operation");

        // The second valid source root changes colour while the first view may
        // still own a GPU submission. Preparation uses a detached scene copy.
        let b_colour = [2.0, 0.8, (1.0 - 2.0 * 0.2126 - 0.8 * 0.7152) / 0.0722];
        let b_paths = fixture.write(b_colour, 100_000);
        let b = fixture.admit(&store, a.generation, 2, &b_paths);
        let mut pin = store
            .pin_current(b.generation)
            .expect("fixture or GPU operation");
        let product_b = cook_render_environment(&mut pin).expect("fixture or GPU operation");
        pin.close().expect("fixture or GPU operation");
        let expected_revision = *product_b.revision().as_bytes();
        let mut candidate_b = host
            .visual_prepare_inputs()
            .prepare(product_b.runtime_product())
            .await
            .expect("fixture or GPU operation");
        let mut published = false;
        for _ in 0..100 {
            if host
                .publish_environment(&mut candidate_b)
                .expect("fixture or GPU operation")
            {
                published = true;
                break;
            }
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(1)),
                })
                .expect("fixture or GPU operation");
        }
        assert!(published, "prepared B must reach the publication barrier");
        let pixels_b = host.capture_pixels().expect("fixture or GPU operation");
        assert_ne!(
            pixels_a, pixels_b,
            "accepted Sun colour must change visible output"
        );
        let tick_b = host
            .drive(Duration::from_millis(34), None)
            .expect("fixture or GPU operation");
        assert_eq!(tick_b.tick, tick_a.tick + 1);
        assert_eq!(tick_b.completed_ticks, 1);
        let mut observed = false;
        for _ in 0..100 {
            if let Some(sample) = host.poll_completed().expect("fixture or GPU operation")
                && sample.tick == tick_b.tick
            {
                assert_eq!(sample.visual_revision, Some(expected_revision));
                observed = true;
                break;
            }
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(1)),
                })
                .expect("fixture or GPU operation");
        }
        assert!(
            observed,
            "published revision must appear in a completed frame"
        );

        let c_paths = fixture.write([1.0; 3], 0);
        let c = fixture.admit(&store, b.generation, 3, &c_paths);
        let mut pin = store
            .pin_current(c.generation)
            .expect("zero-Sun source pin");
        let product_c = cook_render_environment(&mut pin).expect("zero-Sun cook");
        pin.close().expect("zero-Sun pin close");
        let mut candidate_c = host
            .visual_prepare_inputs()
            .prepare(product_c.runtime_product())
            .await
            .expect("zero-Sun GPU preparation");
        let mut published = false;
        for _ in 0..100 {
            if host
                .publish_environment(&mut candidate_c)
                .expect("zero-Sun publish")
            {
                published = true;
                break;
            }
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(1)),
                })
                .expect("zero-Sun GPU wait");
        }
        assert!(published);
        let tick_c = host
            .drive(Duration::from_millis(51), None)
            .expect("zero-Sun drive");
        assert_eq!(tick_c.tick, tick_b.tick + 1);
        let mut observed = false;
        for _ in 0..100 {
            if let Some(sample) = host.poll_completed().expect("zero-Sun report")
                && sample.tick == tick_c.tick
            {
                assert!(!sample.timing_fault);
                if timestamps {
                    let passes = sample.gpu_pass_ns.expect("zero-Sun timestamp report");
                    assert!(passes.iter().all(|pass| pass.is_finite() && *pass >= 0.0));
                }
                observed = true;
                break;
            }
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(1)),
                })
                .expect("zero-Sun GPU wait");
        }
        assert!(observed);
    });
}
