//! The grazing-normal repair on mirrored meshes. A smooth-normal sheet whose
//! interpolated normals fall behind the view must be lifted onto V's side in
//! both classes: the mirrored bucket's `Cw` pipeline has to report its visible
//! faces as `front_facing`, or the repair is skipped and the slope goes dark.

use std::sync::Arc;

use super::{EXTENT, differing, gpu, place};
use crate::synthetic::{
    FrameLighting, PreparedSynthetic, SyntheticLight, SyntheticMesh, SyntheticMeshChoice,
    SyntheticMeshes, SyntheticScene, SyntheticVertex, SyntheticView,
};

/// A square sheet in z = 0 facing +z, wound counter-clockwise from +z, whose
/// vertex normals lean outward along x and away from the viewer
/// (`(±0.8, 0, -0.6)`): every interpolated normal points behind a camera on
/// +z. The sheet is symmetric under x → -x, so mirroring it keeps its surface.
fn sheet() -> SyntheticMesh {
    let vertex = |x: f32, y: f32| SyntheticVertex {
        position: [x, y, 0.0],
        normal: [0.8 * x, 0.0, -0.6],
        uv: [x * 0.5 + 0.5, y * 0.5 + 0.5],
        tangent: [1.0, 0.0, 0.0, 1.0],
    };
    SyntheticMesh {
        vertices: vec![
            vertex(-1.0, -1.0),
            vertex(1.0, -1.0),
            vertex(1.0, 1.0),
            vertex(-1.0, 1.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

/// Red summed over the left and right halves. On the right the repaired
/// normal faces the Sun and the unrepaired one mostly faces away; on the left
/// both face away, so the left half holds only the ambient terms.
fn half_reds(pixels: &[u8]) -> [u64; 2] {
    let width = EXTENT[0] as usize;
    let mut halves = [0; 2];
    for (index, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
        halves[usize::from(index % width >= width / 2)] += u64::from(pixel[0]);
    }
    halves
}

#[test]
fn mirrored_meshes_repair_grazing_normals_like_ordinary_ones() {
    let gpu = gpu();
    let light = SyntheticLight {
        direction_to_light: [0.5, 0.0, 1.0],
        irradiance: [4.0; 3],
    };
    let mut scene = SyntheticScene::grid(1).expect("one instance");
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
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        meshes: vec![sheet()],
        slots: vec![SyntheticMeshChoice::Mesh(0)],
    }));
    let render = |half: [f32; 3]| {
        let mut scene = scene.clone();
        place(&mut scene.instances[0], half, [0.0; 3], 0.0);
        let mut prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
            &gpu.capabilities,
            &gpu.queue,
            &scene,
            FrameLighting::Diagnostic(light),
            1.0,
        ))
        .expect("prepare");
        prepared
            .upload_instances(&gpu.queue, &scene.instances, &[])
            .expect("upload");
        gpu.render_lit(&mut prepared, &scene, FrameLighting::Diagnostic(light))
            .0
    };
    let ordinary = render([0.8, 0.8, 0.8]);
    // Negating the x axis mirrors the slot into the mirrored bucket.
    let mirrored = render([-0.8, 0.8, 0.8]);
    let ([left, right], [_, right_mirrored]) = (half_reds(&ordinary), half_reds(&mirrored));
    let differ = differing(&ordinary, &mirrored);
    eprintln!(
        "grazing sheet: red left {left}, right {right} ordinary, right {right_mirrored} mirrored; \
         {differ} px differ"
    );
    // The Sun lights the repaired right slope well above the ambient-only
    // left; unrepaired, the halves match (about 1.67 against 1.0 here).
    assert!(
        right * 4 > left * 5,
        "the repaired slope is lit: {right} vs {left}"
    );
    assert_eq!(differ, 0, "the mirrored class repairs the same normals");
}
