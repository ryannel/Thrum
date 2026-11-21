use super::*;
use crate::synthetic::{
    SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes, SyntheticVertex,
    mesh_rows::{MeshRowGeometry, MeshRowStatus, MeshRows},
};

fn triangle(scale: f32) -> SyntheticMesh {
    SyntheticMesh {
        vertices: [
            [-scale, -scale, 0.0],
            [scale, -scale, 0.0],
            [0.0, scale, 0.0],
        ]
        .map(|position| SyntheticVertex {
            position,
            normal: [0.0, 0.0, 1.0],
            uv: [0.0; 2],
            tangent: [1.0, 0.0, 0.0, 1.0],
        })
        .to_vec(),
        indices: vec![0, 1, 2],
    }
}

async fn row_host(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    mesh: SyntheticMesh,
) -> (Host, MeshRows) {
    let updates = MeshRows::new(vec![mesh.clone()]);
    let mut scene = SyntheticScene::grid(1).expect("scene");
    scene.sun_shadows = true;
    scene.camera_culling = CullMode::Occlusion;
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        meshes: vec![mesh],
        slots: vec![SyntheticMeshChoice::Mesh(0); scene.instances.len()],
        updates: Some(updates.clone()),
    }));
    let source = SourceTick {
        epoch: 5,
        tick: 0,
        timeline: 3,
    };
    let host = FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile {
                extent,
                ..surface()
            }),
        },
        StaticWorld {
            authority: source,
            publication: source,
        },
        Policy::default(),
        scene,
        0.125,
    )
    .await
    .expect("row host");
    (host, updates)
}

#[test]
fn detached_extent_keeps_ready_rows_and_commits_after_candidate_preparation() {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "extent row inheritance",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        for committed in [false, true] {
            let (mut live, rows) = row_host(&caps, &queue, [64, 64], triangle(0.2)).await;
            let old = live.capture_pixels().expect("old");
            // Prepare against the old geometry, then keep the active host usable.
            let mut candidate = live
                .visual_prepare_inputs()
                .prepare_extent(
                    [96, 48],
                    surface(),
                    FrameLighting::Diagnostic(SyntheticLight::default()),
                    0.125,
                )
                .await
                .expect("detached extent");
            rows.prepare(
                1,
                vec![(0, MeshRowGeometry::new(triangle(0.8), 128 << 20).unwrap())],
            )
            .unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while rows.status(1) != Some(MeshRowStatus::Ready) {
                assert_eq!(live.capture_pixels().expect("write"), old);
                assert!(
                    std::time::Instant::now() < deadline,
                    "row write did not finish"
                );
            }
            assert!(rows.offer(1));
            if committed {
                assert_ne!(live.capture_pixels().expect("commit"), old);
                assert_eq!(rows.status(1), Some(MeshRowStatus::Committed));
            }
            let submitted = rows.report().submitted_bytes;
            assert!(live.publish_visual(&mut candidate).expect("publish extent"));
            let (mut fresh, _) = row_host(&caps, &queue, [96, 48], triangle(0.8)).await;
            let expected = fresh.capture_pixels().expect("fresh extent");
            assert_eq!(live.capture_pixels().expect("inherited extent"), expected);
            assert_eq!(live.capture_pixels().expect("following extent"), expected);
            assert_eq!(rows.status(1), Some(MeshRowStatus::Committed));
            assert_eq!(
                rows.report().submitted_bytes,
                submitted,
                "extent publication rewrote geometry"
            );
        }
    });
}
