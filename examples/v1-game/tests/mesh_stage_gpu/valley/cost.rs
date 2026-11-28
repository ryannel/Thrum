//! Timing runs and a separate geometry accounting replay over the same route.

use std::time::Duration;

use glam::{Mat4, Vec3};
use serde_json::{Value, json};
use thrum_asset::{AssetStore, MeshProduct, Profile};
use thrum_asset_gpu::CookedDrawPath::{Indirect, MeshStages};
use thrum_render::runtime::SurfaceOutput;
use v1_game::GameHost as _;

use super::{Gpu, SunProbe, Valley, Witness, output, read_buffer, store};

const PASSES: [&str; 22] = [
    "contribution compute",
    "synthetic forward opaque",
    "synthetic Sun shadow near",
    "synthetic Sun shadow far",
    "synthetic Sun cascade 2",
    "synthetic Sun cascade 3",
    "synthetic Sun cascade 4",
    "synthetic Sun cascade 5",
    "synthetic Sun cascade 6",
    "synthetic Sun cascade 7",
    "synthetic Sun cascade 8",
    "synthetic Sun cascade 9",
    "synthetic Sun cascade 0 outline 1",
    "synthetic Sun cascade 0 outline 2",
    "synthetic Sun cascade 0 outline 3",
    "synthetic Sun cascade 1 outline 1",
    "synthetic Sun cascade 1 outline 2",
    "synthetic Sun cascade 1 outline 3",
    "synthetic Sun cascade 2 outline 1",
    "synthetic Sun cascade 2 outline 2",
    "synthetic Sun cascade 2 outline 3",
    "synthetic Sun horizon heights",
];

struct Geometry {
    name: String,
    model: Mat4,
    scale: f32,
    product: MeshProduct,
}

fn geometry() -> Vec<Geometry> {
    let (_, config) = v1_game::valley_config(store().into()).expect("authored placements");
    let store = AssetStore::new(store());
    let head = store.read_head().expect("read HEAD").expect("publication");
    let manifest = store.load_manifest(&head).expect("manifest");
    config
        .meshes
        .into_iter()
        .map(|placement| {
            let hash = manifest
                .resolve(&placement.mesh, &[Profile::Portable])
                .expect("mesh")
                .product;
            let product =
                MeshProduct::open(store.map(&hash).expect("map mesh")).expect("valid mesh");
            let model = Mat4::from_cols_array_2d(&placement.transform);
            let scale = model.to_scale_rotation_translation().0.abs().max_element();
            Geometry {
                name: placement.mesh,
                model,
                scale,
                product,
            }
        })
        .collect()
}

fn counts(meshes: &[Geometry], view: Mat4, camera: Option<Vec3>, cull: bool) -> Vec<Value> {
    let w = view.row(3);
    let planes = [
        w + view.row(0),
        w - view.row(0),
        w + view.row(1),
        w - view.row(1),
        w - view.row(2),
        view.row(2),
    ];
    meshes.iter().map(|mesh| {
        let mut kept = [0, 0];
        let mut frustum = [0, 0];
        let mut cone = [0, 0];
        for (bounds, meshlet) in mesh.product.meshlet_bounds().iter().zip(mesh.product.meshlets()) {
            let center = mesh.model.transform_point3(Vec3::from_array(bounds.center));
            let radius = bounds.radius * mesh.scale;
            let result = if cull && planes.iter().any(|plane| {
                plane.dot(center.extend(1.0)) < -radius * plane.truncate().length()
            }) {
                &mut frustum
            } else if cull && bounds.cone_cutoff < 1.0 && camera.is_some_and(|eye| {
                let apex = mesh.model.transform_point3(Vec3::from_array(bounds.cone_apex));
                let axis = mesh.model.transform_vector3(Vec3::from_array(bounds.cone_axis)).normalize();
                (apex - eye).normalize().dot(axis) >= bounds.cone_cutoff
            }) {
                &mut cone
            } else {
                &mut kept
            };
            result[0] += 1;
            result[1] += meshlet.triangle_count;
        }
        json!({"mesh": mesh.name, "kept": kept, "frustum_rejected": frustum, "cone_rejected": cone})
    }).collect()
}

fn matrix(bytes: &[u8]) -> Mat4 {
    let values: [f32; 16] = std::array::from_fn(|i| {
        f32::from_le_bytes(bytes[i * 4..(i + 1) * 4].try_into().expect("matrix field"))
    });
    Mat4::from_cols_array(&values)
}

#[test]
#[ignore = "measure via scripts/mesh_stage_cost.py"]
fn route_cost() {
    let Some(test_gpu) = thrum_test_support::gpu("route_cost", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let width = std::env::var("THRUM_MESH_COST_WIDTH")
        .expect("width")
        .parse()
        .expect("integer width");
    let arm = std::env::var("THRUM_MESH_COST_ARM").expect("arm");
    let count = arm == "counts";
    let path = if arm == "indexed" {
        Indirect
    } else {
        MeshStages
    };
    // Keep both timing arms on the same hardware generation profile.
    let gpu = Gpu::new(&test_gpu, true, "apple-tile", |_| {});
    let extent = [width, width * 9 / 16];
    let mut valley = Valley::new(&gpu, path, true, extent, 12.0, SunProbe::None);
    let route = valley.install_route(12.0);
    let meshes = geometry();
    let mut frames = Vec::new();
    let witness = count.then(|| {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("route shadow view witness"),
            size: ((PASSES.len() - 2) * 80) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    });
    let readback = count.then(|| {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("route shadow view readback"),
            size: ((PASSES.len() - 2) * 80) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    });
    for relative_tick in 0..=route.keys.last().expect("route end").tick {
        let now = Duration::from_secs_f64(valley.host.route_step_ms(0.5) / 1000.0);
        if let (Some(witness), Some(readback)) = (&witness, &readback) {
            let mut encoded = valley
                .host
                .advance(now)
                .expect("advance")
                .encode_scene()
                .expect("encode");
            let frame_extent = encoded.frame_extent().expect("encoded view");
            assert_eq!(
                encoded.copy_shadow_view_witness(witness),
                Some(witness.size())
            );
            assert!(
                encoded
                    .submit(Some(&SurfaceOutput {
                        view: &valley.drawable,
                        extent,
                    }))
                    .expect("submit")
                    .submitted
            );
            let mut copy = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            copy.copy_buffer_to_buffer(witness, 0, readback, 0, witness.size());
            let bytes = read_buffer(&gpu, copy, readback);
            let sample = valley.complete();
            assert_eq!(sample.tick, route.origin_tick + relative_tick);
            let camera = route.pose(sample.tick, sample.presentation_alpha).eye;
            let forward = counts(
                &meshes,
                Mat4::from_cols_array_2d(&frame_extent.view_projection),
                Some(Vec3::from_array(camera)),
                true,
            );
            let shadows: Vec<_> = PASSES[2..]
                .iter()
                .zip(bytes.as_chunks::<80>().0)
                .map(|(pass, bytes)| {
                    let view = matrix(bytes);
                    json!({"pass": pass,
                    "mesh_stages": counts(&meshes, view, None, true),
                    "indexed": counts(&meshes, view, None, false)})
                })
                .collect();
            frames.push(
                json!({"route_tick": relative_tick, "forward_both": forward, "shadows": shadows}),
            );
        } else {
            let (sample, _) = valley.frame(now, Witness::None);
            assert_eq!(sample.tick, route.origin_tick + relative_tick);
            let gpu_passes = sample.gpu_pass_ns.expect("measured Metal timestamps");
            let span = gpu_passes
                .timed_pass_span_nanoseconds()
                .expect("timed-pass span through output");
            assert!(span > 0.0 && sample.surface_presented);
            let timings: serde_json::Map<_, _> = PASSES
                .into_iter()
                .map(|pass| {
                    let time = gpu_passes.get(pass);
                    if pass != PASSES[0] || path == Indirect {
                        assert!(time.is_some(), "{pass}: {:?}", gpu_passes.values());
                    } else {
                        assert!(time.is_none(), "mesh-stage arm ran an indexed cooked cull");
                    }
                    (pass.to_owned(), json!(time))
                })
                .collect();
            let stats = valley.stats.lock().expect("draw observations");
            let bytes = stats.draw_residency;
            let buffer_bytes = stats.mesh_buffer_bytes
                + bytes.instances
                + bytes.materials
                + bytes.indirect_arguments
                + bytes.cull_uniforms
                + bytes.visibility
                + bytes.volume_uniform;
            assert_eq!(stats.sun_caster_draws.len(), PASSES[2..].len());
            let caster_draws: serde_json::Map<_, _> = PASSES[2..]
                .iter()
                .zip(&stats.sun_caster_draws)
                .map(|(&pass, &draws)| (pass.to_owned(), json!(draws)))
                .collect();
            frames.push(json!({
                "route_tick": relative_tick, "submission": sample.submission_order,
                "timed_pass_span_ns": span, "frame_cpu_ns": (sample.upload_cpu + sample.encode_cpu).as_nanos(),
                "upload_cpu_ns": sample.upload_cpu.as_nanos(),
                "encode_cpu_ns": sample.encode_cpu.as_nanos(),
                "view_update_cpu_ns": sample.view_update_ns,
                "passes_ns": timings, "surface_consumer_ns": sample.gpu_present_ns,
                "forward_draws": stats.mesh_stage_draws + stats.indexed_indirect_draws + stats.indexed_direct_draws,
                "draws_per_sun_map": caster_draws,
                "mesh_buffer_bytes": stats.mesh_buffer_bytes, "mesh_stage_bytes": stats.mesh_stage_bytes,
                "resident_draw_buffer_bytes": buffer_bytes,
                "resident_draw_resource_bytes": buffer_bytes + bytes.sampled_textures,
                "draw_resource_breakdown": {
                    "instances": bytes.instances, "materials": bytes.materials,
                    "indirect_arguments": bytes.indirect_arguments, "cull_uniforms": bytes.cull_uniforms,
                    "visibility": bytes.visibility, "volume_uniform": bytes.volume_uniform,
                    "sampled_textures": bytes.sampled_textures,
                },
            }));
        }
    }
    let record = json!({
        "arm": arm, "path": format!("{path:?}"), "recipe": "AppleTile", "extent": extent,
        "adapter": gpu.device.adapter_info().name, "registry_digest": gpu.capabilities.registry_digest(),
        "frames": frames,
    });
    std::fs::write(
        output().join("route.json"),
        serde_json::to_vec(&record).expect("record encoding"),
    )
    .expect("route record");
}
