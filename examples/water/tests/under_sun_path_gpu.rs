//! A low Sun casts one refracted shadow in the bed and the water above it.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn the_bed_and_signed_march_share_the_refracted_shadow() {
    use support::optics::{EXTENT, Panel, Rig, attach, camera, water_configured};
    use thrum_render::{atmosphere::AtmosphereFrame, synthetic::FrameLighting};
    use thrum_test_support::readback::{Mip, texture};
    use thrum_water::{Optics, Profile};

    let Some(gpu) = thrum_test_support::gpu("refracted Sun shadow", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let angle = 20.0_f32.to_radians();
    let direction = [angle.cos(), angle.sin(), 0.0];
    let light = FrameLighting::Atmosphere(
        AtmosphereFrame::new(direction, 80_000.0, [1.0; 3], 1.0, 1).unwrap(),
    );
    let view_at = |x, offset| camera([x, -5.75, offset], [x, -6.0, 0.0], 0.65, 1.0);
    let stations = [-9.0, -8.5, -8.0, -7.5, -7.0, -6.5, -6.0, -5.5, -5.0, -4.5, -4.0, -17.0];
    let bed = Panel { centre: [-7.0, -6.02, 0.0], half_size: [28.0, 0.02, 12.0], radiant: false };
    let caster = Panel { centre: [0.0, 0.25, 0.0], half_size: [1.25, 0.1, 12.0], radiant: false };
    let render = |caster_on, shafts, offset| {
        let mut water = water_configured(None, Profile {
            caustics: true, shafts, optics: shafts.then_some(Optics::default()),
            underwater: shafts, deep_scatter: [0.025, 0.075, 0.13], ..Profile::default()
        });
        let witness = water.tap_shafts();
        let panels = [bed, caster];
        let mut rig = Rig::with_scene(
            support::device(&gpu.fresh_adapter()), &view_at(stations[0], offset), light,
            if caster_on { &panels } else { &panels[..1] }, None, EXTENT,
            |scene| {
                scene.sun_shadows = true;
                scene.sun_path = water.sun_path_provider();
            },
        );
        attach(&mut rig, water);
        rig.host.set_quality_knob("water.caustic_cell_scale", 3).unwrap();
        rig.host.set_quality_knob("water.caustic_rungs", 5).unwrap();
        stations.map(|x| {
            rig.host.source_mut().view = view_at(x, offset);
            if let Some(image) = witness.texture() {
                rig.frame();
                let center = (27 * u64::from(image.width()) + 48) * 8;
                let bytes = texture(&rig.device, &rig.queue, &image, Mip::level(0), center..center+8);
                support::half_value(u16::from_le_bytes([bytes[2], bytes[3]]))
            } else {
                let (_, color) = rig.scene_colour();
                color[54 * 192 + 96][1]
            }
        })
    };
    let centre = |clear: [f32; 12], shadow: [f32; 12]| {
        let differences = std::array::from_fn::<_, 12, _>(|index| clear[index] - shadow[index]);
        let strongest = differences.into_iter().fold(0.0, f32::max);
        assert!(strongest > 0.0001, "no submitted shadow: {differences:?}");
        assert!(differences[11].abs() < strongest * 0.1, "shadow followed the Sun's air ray");
        let mut weighted = 0.0;
        let mut weight = 0.0;
        for (x, difference) in stations.into_iter().zip(differences) {
            if difference > strongest * 0.5 {
                weighted += x * difference;
                weight += difference;
            }
        }
        assert!(weight > strongest * 1.5, "shadow occupies fewer than two stations");
        (weighted / weight, differences)
    };
    // The snapped window's centre, fade and exterior all cross the same bed stations.
    for offset in [0.02, 8.0, 11.0] {
        let (bed_x, bed_samples) = centre(render(false, false, offset), render(true, false, offset));
        let (water_x, water_samples) = centre(render(false, true, offset), render(true, true, offset));
        let tolerance = 0.45;
        eprintln!("camera Z={offset} m, bed shadow x={bed_x:.3} m, signed water shadow x={water_x:.3} m, tolerance={tolerance} m");
        eprintln!("distances along bed: {stations:?}");
        eprintln!("loss of direct light on bed: {bed_samples:?}");
        eprintln!("loss of scattered light over bed: {water_samples:?}");
        assert!((bed_x - water_x).abs() < tolerance, "bed and water shadows diverged");
    }
}
}
