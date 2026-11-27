//! Counterexample for sharing the main camera's finite height-map support
//! with an unrestricted secondary camera. Such a request is now refused;
//! the declared-position test also holds every admitted vertical to one map.

use super::*;
use crate::synthetic::view::Projection;
use crate::synthetic::{
    CaptureSource, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticScene,
};

#[test]
fn height_anchor_uses_declared_position_on_a_texel_edge() {
    let cameras = [
        ([16.0, 10.0, 0.0], [0.0, 0.0, -1.0]),
        ([16.0, 10.0, 0.0], [-2.0, -1.7, -1.0]),
        ([16.0, -70.0, 0.0], [-2.0, -1.6, -1.0]),
        ([16.0, 0.0, 0.0], [-2.0, 1.7, -1.0]),
    ];
    let casters = [instance(0, [100.0, 125.0, -3000.0], [125.0, 125.0, 400.0])];
    let mut reference = None;
    let mut recovered_layouts = Vec::new();
    for (eye, direction) in cameras {
        let view = SyntheticView::perspective(
            eye,
            (Vec3::from_array(eye) + Vec3::from_array(direction)).to_array(),
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            6000.0,
        )
        .expect("camera");
        let inputs =
            FitInputs::new(&view, (&casters, None), Vec3::ONE.normalize()).expect("scene inputs");
        let fitted = inputs.fit(None).expect("scene fit");
        let layout = (
            fitted.matrices[HEIGHT_LAYER].to_cols_array(),
            (fitted.samples.height_view_proj, fitted.samples.horizon),
        );
        if let Some(expected) = &reference {
            assert_eq!(
                &layout, expected,
                "same vertical, identical layout and address"
            );
        } else {
            reference = Some(layout);
        }
        let Projection::Perspective { view: matrix, .. } = view.projection() else {
            panic!("perspective")
        };
        let recovered = matrix.inverse().w_axis.truncate();
        let old = height_matrix(recovered, inputs.caster_height)
            .expect("old layout")
            .0
            .to_cols_array();
        if !recovered_layouts.contains(&old) {
            recovered_layouts.push(old);
        }
        eprintln!("height_anchor declared={eye:?}, recovered={recovered:?}");
    }
    assert!(
        recovered_layouts.len() > 1,
        "matrix-derived positions snap into different texels"
    );
    eprintln!(
        "height_anchor: one declared layout; {} matrix-derived layouts",
        recovered_layouts.len()
    );
}

fn instance(slot: u32, center: [f32; 3], half: [f32; 3]) -> InstanceRecord {
    let mut instance = InstanceRecord::identity(slot, 0);
    for axis in 0..3 {
        instance.current[axis][axis] = half[axis];
        instance.normal[axis][axis] = half[axis].recip();
        instance.current[3][axis] = center[axis];
    }
    instance.previous = instance.current;
    instance
}

fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
) -> Vec<[f32; 4]> {
    let target = OffscreenTarget::with_capture_source(
        device,
        prepared,
        128,
        128,
        CaptureSource::SceneLinear,
    )
    .expect("scene target");
    let mut readback = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(&mut encoder, 1, &target, None, &mut readback)
        .expect("ordinary scene graph");
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .expect("readback complete");
    let mut bytes = Vec::new();
    assert!(readback.try_collect_into(1, &mut bytes).expect("pixels"));
    bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|pixel| {
            std::array::from_fn(|channel| {
                half::f16::from_le_bytes([pixel[channel * 2], pixel[channel * 2 + 1]]).to_f32()
            })
        })
        .collect()
}

#[test]
fn a_shared_main_height_map_cannot_cover_an_unrestricted_view_camera() {
    let Some((device, queue)) = observer::device(
        "a_shared_main_height_map_cannot_cover_an_unrestricted_view_camera",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let mut scene = SyntheticScene::grid(2).expect("scene");
        scene.sun_shadows = true;
        scene.materials[0].base_rgba = [0.8, 0.8, 0.8, 1.0];
        scene.instances = vec![
            instance(0, [0.0, -0.5, -3000.0], [600.0, 0.5, 500.0]),
            instance(1, [250.0, 125.0, -3000.0], [125.0, 125.0, 400.0]),
        ];
        scene.view = SyntheticView::perspective(
            [0.0, 1000.0, 0.0],
            [0.0, 0.0, -3000.0],
            [0.0, 1.0, 0.0],
            0.6,
            1.0,
            0.1,
            6000.0,
        )
        .expect("requested camera");
        let direction = Vec3::new(1.0, 1.0, 0.0).normalize();
        let light = SyntheticLight {
            direction_to_light: direction.to_array(),
            irradiance: [1.0; 3],
        };
        let mut prepared = PreparedSynthetic::prepare_with_light(&device, &queue, 1, &scene, light)
            .await
            .expect("ordinary reference preparation");
        let reference = render(&device, &queue, &prepared);
        // Preserve the requested camera, both cascades, all world data and
        // shading. Only layer 2 and its address record use the main camera's
        // height-map centre, exactly the product the design says to share.
        let displaced = SyntheticView::perspective(
            [40000.0, 1000.0, 0.0],
            [40000.0, 0.0, -3000.0],
            [0.0, 1.0, 0.0],
            0.6,
            1.0,
            0.1,
            6000.0,
        )
        .expect("main camera");
        let main = FitInputs::new(&displaced, (&scene.instances, None), direction)
            .expect("main inputs")
            .fit(None)
            .expect("main fit");
        let shadow = prepared.shadow.as_mut().expect("Sun shadows");
        shadow.fitted.matrices[HEIGHT_LAYER] = main.matrices[HEIGHT_LAYER];
        shadow.fitted.samples.height_view_proj = main.samples.height_view_proj;
        shadow.fitted.samples.horizon = main.samples.horizon;
        let shared = render(&device, &queue, &prepared);
        let mut changed = 0;
        let mut max_steps = 0.0_f32;
        let mut example = None;
        for (index, (expected, actual)) in reference.iter().zip(&shared).enumerate() {
            if expected[0].to_bits() == actual[0].to_bits() {
                continue;
            }
            let stored = half::f16::from_f32(expected[0]);
            let step = (half::f16::from_bits(stored.to_bits() + 1).to_f32() - expected[0]).abs();
            let steps = (actual[0] - expected[0]).abs() / step;
            if steps > 1.0 {
                changed += 1;
            }
            if steps > max_steps {
                max_steps = steps;
                example = Some((index % 128, index / 128, expected[0], actual[0], step));
            }
        }
        eprintln!(
            "SCENE-VIEWS shared height-map counterexample: changed pixels={changed}, max stored steps={max_steps}, example (x,y,reference,shared,step)={example:?}"
        );
        assert!(
            changed > 10 && max_steps > 10.0,
            "the shared map must visibly miss the ordinary main-view reference"
        );
    });
}
