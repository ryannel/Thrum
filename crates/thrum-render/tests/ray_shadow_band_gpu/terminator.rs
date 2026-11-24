#[cfg(target_os = "macos")]
use crate::{ray_shadow_support, ray_support::support};
#[cfg(target_os = "macos")]
use glam::Vec3;
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use thrum_render::{
    abi::InstanceRecord,
    synthetic::{
        SunShadowMethod, SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes, SyntheticScene,
        SyntheticVertex, SyntheticView,
    },
};

#[cfg(target_os = "macos")]
fn quad(mesh: &mut SyntheticMesh, points: [Vec3; 4], normals: [Vec3; 4], reverse: bool) {
    let start = u32::try_from(mesh.vertices.len()).expect("small fixture");
    mesh.vertices
        .extend(
            points
                .into_iter()
                .zip(normals)
                .map(|(position, normal)| SyntheticVertex {
                    position: position.to_array(),
                    normal: normal.to_array(),
                    uv: [0.0; 2],
                    tangent: [0.0, 0.0, 1.0, 1.0],
                }),
        );
    let indices = if reverse {
        [0, 2, 1, 1, 2, 3]
    } else {
        [0, 1, 2, 1, 3, 2]
    };
    mesh.indices.extend(indices.map(|index| start + index));
}

#[cfg(target_os = "macos")]
fn empty_mesh() -> SyntheticMesh {
    SyntheticMesh {
        vertices: vec![],
        indices: vec![],
    }
}

#[cfg(target_os = "macos")]
fn prism(open_rim: bool) -> SyntheticMesh {
    let radial = |side: u16| {
        let (y, x) = (30.0 * f32::from(side) - 15.0).to_radians().sin_cos();
        Vec3::new(x, y, 0.0)
    };
    let mut mesh = empty_mesh();
    for side in 0..12 {
        // Side 1 is the far face that the receiver's Sun ray crosses.
        if open_rim && side == 1 {
            continue;
        }
        let a = radial(side);
        let b = radial((side + 1) % 12);
        quad(
            &mut mesh,
            [a - Vec3::Z, b - Vec3::Z, a + Vec3::Z, b + Vec3::Z],
            [a, b, a, b],
            false,
        );
    }
    mesh
}

#[derive(Clone, Copy, Debug)]
#[cfg(target_os = "macos")]
struct Sheet {
    distance: f32,
    joined: bool,
    reverse: bool,
}

#[cfg(target_os = "macos")]
fn scene(receiver: Vec3, sun: Vec3, sheet: Option<Sheet>, open_rim: bool) -> SyntheticScene {
    let mut meshes = vec![prism(open_rim)];
    if let Some(sheet) = sheet {
        let centre = receiver + sun * sheet.distance;
        let width = sun.cross(Vec3::Z) * 0.08;
        let depth = Vec3::Z * 0.12;
        let points = [
            centre - width - depth,
            centre + width - depth,
            centre - width + depth,
            centre + width + depth,
        ];
        let normals = [if sheet.reverse { sun } else { -sun }; 4];
        if !sheet.joined {
            meshes.push(empty_mesh());
        }
        quad(
            meshes.last_mut().expect("mesh"),
            points,
            normals,
            sheet.reverse,
        );
    }
    let count = u32::try_from(meshes.len()).expect("fixture");
    let mut scene = SyntheticScene::grid(count).expect("scene");
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        slots: (0..count).map(SyntheticMeshChoice::Mesh).collect(),
        meshes,
        updates: None,
    }));
    scene.instances = (0..count)
        .map(|slot| InstanceRecord::identity(slot, 0))
        .collect();
    scene.view = SyntheticView::perspective(
        (receiver + Vec3::X * 2.5).to_array(),
        receiver.to_array(),
        Vec3::Y.to_array(),
        0.42,
        1.0,
        0.05,
        12.0,
    )
    .expect("terminator view");
    scene.sun_shadows = true;
    scene.sun_shadow_method = SunShadowMethod::Traced;
    scene
}

thrum_test_support::macos_gpu_test! {
fn terminator_excuses_only_the_first_crossing() {
    let Some(gpu) = support::gate("forward Sun terminator and joined sheet") else {
        return;
    };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let receiver = Vec3::new(15.0_f32.to_radians().cos(), 0.23, 0.0);
    let (y, x) = 94.0_f32.to_radians().sin_cos();
    let sun = Vec3::new(x, y, 0.0);
    let lighting = ray_shadow_support::sun(sun, 8_000.0);
    // The triangle faces +X, but the radial normal faces the Sun. The far
    // face is about 34 mm ahead. Both sheets follow it and bracket the reach.
    let clear = receiver + Vec3::Z * 0.28;
    let distances = [0.25, 3.0];
    let cases = [false, true].into_iter().flat_map(|joined| {
        [false, true].into_iter().flat_map(move |reverse| {
            distances.map(move |distance| {
                let sheet = Sheet {
                    distance,
                    joined,
                    reverse,
                };
                (false, Some(sheet), false)
            })
        })
    });
    // With the far face removed, the joined back-face sheet becomes the
    // first hit. The passage excuses it only at the near distance. This
    // control makes a change in reach visible in the same forward operation.
    let reach_controls = distances.into_iter().zip([true, false]).map(|(distance, lit)| {
        let sheet = Sheet {
            distance,
            joined: true,
            reverse: true,
        };
        (true, Some(sheet), lit)
    });
    for (open_rim, sheet, lit) in std::iter::once((false, None, true))
        .chain(cases)
        .chain(reach_controls)
    {
        let scene = scene(receiver, sun, sheet, open_rim);
        let (prepared, target) =
            ray_shadow_support::prepare(&device, &caps, &queue, &scene, lighting);
        assert_eq!(prepared.shadow_selection().sun, SunShadowMethod::Traced);
        let image = support::submit_bytes(&device, &queue, &prepared, &target);
        let middle = ray_shadow_support::sample(&scene.view, &image, receiver);
        let beside = ray_shadow_support::sample(&scene.view, &image, clear);
        let case = format!("open rim {open_rim}, sheet {sheet:?}");
        assert!(beside[0] > 32, "{case}: beside {beside:?}");
        if lit {
            assert!(middle[0] > 32, "{case}: expected lit middle, got {middle:?}");
        } else {
            assert!(middle[0] <= 2, "{case}: expected dark middle, got {middle:?}");
        }
        println!("{case}: middle {middle:?}, beside {beside:?}");
    }
}
}
