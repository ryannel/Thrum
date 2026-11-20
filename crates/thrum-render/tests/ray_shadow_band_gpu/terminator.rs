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
fn prism() -> SyntheticMesh {
    let radial = |side: u16| {
        let (y, x) = (30.0 * f32::from(side) - 15.0).to_radians().sin_cos();
        Vec3::new(x, y, 0.0)
    };
    let mut mesh = empty_mesh();
    for side in 0..12 {
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
fn scene(receiver: Vec3, sun: Vec3, sheet: Option<Sheet>) -> SyntheticScene {
    let mut meshes = vec![prism()];
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
    // The triangle faces +X, but the radial normal faces the Sun. The opposite
    // crossing is about 34 mm ahead, with about 90 mm of reach. Both sheets are
    // beyond the crossing and only the 60 mm sheet is in reach.
    let clear = receiver + Vec3::Z * 0.28;
    let cases = [false, true].into_iter().flat_map(|joined| {
        [false, true].into_iter().flat_map(move |reverse| {
            [0.06, 0.10].map(move |distance| {
                Some(Sheet {
                    distance,
                    joined,
                    reverse,
                })
            })
        })
    });
    for sheet in std::iter::once(None).chain(cases) {
        let scene = scene(receiver, sun, sheet);
        let (prepared, target) =
            ray_shadow_support::prepare(&device, &caps, &queue, &scene, lighting);
        assert_eq!(prepared.shadow_selection().sun, SunShadowMethod::Traced);
        let image = support::submit_bytes(&device, &queue, &prepared, &target);
        let middle = ray_shadow_support::sample(&scene.view, &image, receiver);
        let beside = ray_shadow_support::sample(&scene.view, &image, clear);
        assert!(beside[0] > 32, "{sheet:?}: beside {beside:?}");
        if sheet.is_some() {
            assert!(middle[0] <= 2, "{sheet:?}: middle {middle:?}");
        } else {
            assert!(middle[0] > 32, "unobstructed middle {middle:?}");
        }
        println!("{sheet:?}: middle {middle:?}, beside {beside:?}");
    }
}
}
