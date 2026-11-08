//! Sun casters on the bucket route: every admitted caster draws from the arena
//! with its own shape and extent, on screen or not, in both Sun cascades and
//! the top-down height layer, and Frustum's camera culling never removes one.

use super::super::super::mesh_arena::{MeshId, MeshVertex};
use super::super::MeshSelection;
use super::{EXTENT, Gpu, differing, gpu, place};
use crate::synthetic::SyntheticView;
use crate::synthetic::{FrameLighting, PreparedSynthetic, SyntheticLight, SyntheticScene};
use crate::visibility::CullMode;

/// A square pyramid three times taller than the unit cube: base half-width 1
/// at y = -3, apex at y = 3. Its silhouette is not a box, and its bounds reach
/// past the unit cube the legacy fit assumes.
fn pyramid() -> (Vec<MeshVertex>, Vec<u32>) {
    let base = [
        [-1.0, -3.0, -1.0],
        [1.0, -3.0, -1.0],
        [1.0, -3.0, 1.0],
        [-1.0, -3.0, 1.0],
    ];
    let apex = [0.0, 3.0, 0.0];
    let mut faces: Vec<[[f32; 3]; 3]> = (0..4)
        .map(|side| [base[side], apex, base[(side + 1) % 4]])
        .collect();
    faces.push([base[0], base[1], base[2]]);
    faces.push([base[0], base[2], base[3]]);
    let mut vertices = Vec::new();
    for mut face in faces {
        let sub = |a: [f32; 3], b: [f32; 3]| std::array::from_fn::<f32, 3, _>(|i| a[i] - b[i]);
        let (u, v) = (sub(face[1], face[0]), sub(face[2], face[0]));
        let mut normal = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        // Counter-clockwise seen from outside: the normal points away from
        // the pyramid's centre (0, -1.5, 0).
        let centroid: [f32; 3] =
            std::array::from_fn(|i| (face[0][i] + face[1][i] + face[2][i]) / 3.0);
        let outward =
            centroid[0] * normal[0] + (centroid[1] + 1.5) * normal[1] + centroid[2] * normal[2];
        if outward < 0.0 {
            face.swap(1, 2);
            normal = normal.map(|value| -value);
        }
        let length = normal.iter().map(|value| value * value).sum::<f32>().sqrt();
        for position in face {
            vertices.push(MeshVertex {
                position,
                normal: normal.map(|value| value / length),
                uv: [0.5; 2],
                tangent: [1.0, 0.0, 0.0, 1.0],
            });
        }
    }
    let indices = (0..u32::try_from(vertices.len()).expect("18 vertices")).collect();
    (vertices, indices)
}

const PYRAMID: MeshId = MeshId::for_test(2);

impl Gpu {
    fn prepare_casters(
        &self,
        scene: &SyntheticScene,
        slots: Option<Vec<MeshId>>,
        light: SyntheticLight,
    ) -> PreparedSynthetic {
        let lighting = FrameLighting::Diagnostic(light);
        pollster::block_on(async {
            match slots {
                Some(slots) => {
                    let selection = MeshSelection {
                        meshes: vec![pyramid()],
                        slots,
                    };
                    PreparedSynthetic::prepare_meshes(
                        &self.capabilities,
                        &self.queue,
                        scene,
                        lighting,
                        &selection,
                    )
                    .await
                }
                None => {
                    PreparedSynthetic::prepare_with_capabilities(
                        &self.capabilities,
                        &self.queue,
                        scene,
                        lighting,
                        1.0,
                    )
                    .await
                }
            }
        })
        .expect("prepare")
    }

    /// One shadowed frame: pixels, the culled route's drawn count, and the
    /// casters the fit had to clamp.
    fn cast(
        &self,
        scene: &SyntheticScene,
        slots: Option<Vec<MeshId>>,
        light: SyntheticLight,
    ) -> (Vec<u8>, Option<u32>, usize) {
        let mut prepared = self.prepare_casters(scene, slots, light);
        let clamped = prepared
            .upload_instances(&self.queue, &scene.instances, &[])
            .expect("steady upload")
            .clamped_shadow_casters;
        let (pixels, drawn) =
            self.render_lit(&mut prepared, scene, FrameLighting::Diagnostic(light));
        (pixels, drawn, clamped)
    }
}

/// Pixels the shadow darkened by more than `margin` in red.
fn shadow_mask(lit: &[u8], shaded: &[u8], margin: u8) -> Vec<bool> {
    lit.as_chunks::<4>()
        .0
        .iter()
        .zip(shaded.as_chunks::<4>().0)
        .map(|(lit, shaded)| shaded[0].saturating_add(margin) < lit[0])
        .collect()
}

/// Rows of the image holding any masked pixel.
fn rows(mask: &[bool]) -> usize {
    mask.chunks(EXTENT[0] as usize)
        .filter(|row| row.iter().any(|&shadowed| shadowed))
        .count()
}

#[test]
fn an_off_screen_pyramid_casts_its_own_shape_in_the_cascades() {
    let gpu = gpu();
    let light = SyntheticLight {
        direction_to_light: [1.0, 0.0, 1.0],
        irradiance: [4.0; 3],
    };
    let mut scene = SyntheticScene::grid(2).expect("two instances");
    scene.view = SyntheticView::perspective(
        [0.0, 0.0, 3.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        1.0,
        16.0 / 9.0,
        0.1,
        20.0,
    )
    .expect("view");
    // A receiver facing the camera, and a caster left of the view frustum
    // whose shadow falls on the receiver's right half.
    place(
        &mut scene.instances[0],
        [2.2, 1.2, 0.01],
        [0.0, 0.0, 0.0],
        0.0,
    );
    place(&mut scene.instances[1], [0.2; 3], [2.4, 0.0, 1.0], 0.0);
    let pyramid = vec![MeshId::CUBE, PYRAMID];
    let cube = vec![MeshId::CUBE; 2];

    let (lit, _, _) = gpu.cast(&scene, Some(pyramid.clone()), light);
    scene.sun_shadows = true;
    let (direct, _, clamped) = gpu.cast(&scene, Some(pyramid.clone()), light);
    let (cube_route, _, _) = gpu.cast(&scene, Some(cube), light);
    let (legacy, _, _) = gpu.cast(&scene, None, light);
    scene.camera_culling = CullMode::Frustum;
    let (frustum, drawn, _) = gpu.cast(&scene, Some(pyramid), light);

    let pyramid_mask = shadow_mask(&lit, &direct, 25);
    let cube_mask = shadow_mask(&lit, &cube_route, 25);
    let (pyramid_rows, cube_rows) = (rows(&pyramid_mask), rows(&cube_mask));
    eprintln!(
        "cascades: pyramid shadow {} px over {pyramid_rows} rows, cube {} px over {cube_rows} rows",
        pyramid_mask.iter().filter(|&&shadowed| shadowed).count(),
        cube_mask.iter().filter(|&&shadowed| shadowed).count(),
    );
    assert_eq!(drawn, Some(1), "Frustum draws only the receiver");
    assert_eq!(
        differing(&direct, &frustum),
        0,
        "Frustum keeps the off-screen caster"
    );
    assert_eq!(
        differing(&legacy, &cube_route),
        0,
        "cube casters match the legacy route"
    );
    assert_eq!(clamped, 0, "the fit encloses the pyramid's extent");
    assert!(cube_rows > 5, "the cube casts a shadow");
    // The pyramid is three times the cube's height, and tapers.
    assert!(
        pyramid_rows > cube_rows * 2,
        "pyramid {pyramid_rows} rows, cube {cube_rows}"
    );
    assert_ne!(pyramid_mask, cube_mask);

    // Containment uses the mesh's bounds too. Raise the pyramid 1.7 m after
    // the prepared fit: its apex (2.3 m) leaves the height layer's range (the
    // receiver's 1.2 m top plus 1 m of padding), while a unit cube's top
    // (1.9 m) would not.
    let mut prepared = gpu.prepare_casters(&scene, Some(vec![MeshId::CUBE, PYRAMID]), light);
    let mut raised = scene.instances.clone();
    raised[1].current[3][1] = 1.7;
    raised[1].previous = raised[1].current;
    let report = prepared
        .upload_instances(&gpu.queue, &raised, &[1])
        .expect("raised caster");
    assert_eq!(
        report.clamped_shadow_casters, 1,
        "the raised apex is clamped"
    );
}

#[test]
fn a_distant_off_screen_pyramid_shades_through_the_height_layer() {
    let gpu = gpu();
    // A low Sun from +x: receivers beyond the cascades' reach march the
    // height layer toward it.
    let light = SyntheticLight {
        direction_to_light: [1.0, 0.05, 0.0],
        irradiance: [4.0; 3],
    };
    let mut scene = SyntheticScene::grid(2).expect("two instances");
    scene.view = SyntheticView::perspective(
        [0.0, 150.0, 0.0],
        [0.0, 150.0, -1000.0],
        [0.0, 1.0, 0.0],
        1.0,
        16.0 / 9.0,
        1.0,
        3000.0,
    )
    .expect("view");
    // A plate 1 km away, turned toward the Sun, at 150 m. The caster stands
    // 1.5 km to its right, outside the view: as a cube its top is 100 m, as
    // the pyramid 300 m.
    let yaw = std::f32::consts::FRAC_PI_4;
    place(
        &mut scene.instances[0],
        [150.0, 40.0, 1.0],
        [0.0, 150.0, -1000.0],
        yaw,
    );
    place(
        &mut scene.instances[1],
        [200.0, 100.0, 200.0],
        [1500.0, 0.0, -1000.0],
        0.0,
    );
    let pyramid = vec![MeshId::CUBE, PYRAMID];
    let cube = vec![MeshId::CUBE; 2];
    scene.sun_shadows = true;
    let (pyramid_direct, _, clamped) = gpu.cast(&scene, Some(pyramid.clone()), light);
    let (cube_direct, _, _) = gpu.cast(&scene, Some(cube), light);
    let (legacy, _, _) = gpu.cast(&scene, None, light);
    scene.camera_culling = CullMode::Frustum;
    let (pyramid_frustum, drawn, _) = gpu.cast(&scene, Some(pyramid), light);

    let center = ((EXTENT[1] / 2 * EXTENT[0] + EXTENT[0] / 2) * 4) as usize;
    let (pyramid_red, cube_red) = (pyramid_direct[center], cube_direct[center]);
    eprintln!(
        "height layer: plate centre red {pyramid_red} under the pyramid, {cube_red} under the cube"
    );
    assert_eq!(drawn, Some(1), "Frustum draws only the plate");
    assert_eq!(differing(&pyramid_direct, &pyramid_frustum), 0);
    assert_eq!(differing(&legacy, &cube_direct), 0);
    assert_eq!(clamped, 0, "the height fit encloses the pyramid's top");
    assert!(cube_red > 60, "the cube's top is below the plate: lit");
    assert!(
        u16::from(pyramid_red) + 25 < u16::from(cube_red),
        "the pyramid's top blocks the low Sun: {pyramid_red} vs {cube_red}"
    );
}
