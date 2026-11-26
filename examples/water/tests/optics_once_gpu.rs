//! Metal's stored panel and bed contributions across exposure and air changes.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
use support::optics::{EXTENT, Panel, Rig, attach, camera, sun, water};

#[cfg(target_os = "macos")]
const AEROSOL: [f32; 2] = [0.0, 0.000_55];
#[cfg(target_os = "macos")]
const SCALE: [f32; 2] = [1.0 / 32768.0, 1.0 / 2048.0];

#[cfg(target_os = "macos")]
fn arm(
    gpu: &thrum_test_support::Gpu,
    reflection: bool,
    aerosol: f32,
    scale: f32,
    radiant: bool,
    wet: bool,
) -> (f32, Vec<[f32; 3]>) {
    let panel = if reflection {
        Panel {
            centre: [0.0, 0.76, 3000.0],
            half_size: [4.0, 0.52, 0.1],
            radiant,
        }
    } else {
        Panel {
            centre: [0.0, -0.30, 2990.0],
            half_size: [7.0, 0.15, 90.0],
            radiant,
        }
    };
    let view = camera([0.0, 140.0, 0.0], [0.0, 0.0, 2992.0], 0.0035, scale);
    let mut rig = Rig::new(gpu, &view, sun(140.0, aerosol), &[panel], None);
    if wet {
        let bed = (!reflection).then_some((-0.45, [-425.0, 2520.0], 850.0));
        let bounds = attach(&mut rig, water(bed, true));
        if reflection {
            assert!(bounds.publish(&[panel.bounds()]));
        }
    }
    rig.scene_colour()
}

#[cfg(target_os = "macos")]
fn isolated(
    gpu: &thrum_test_support::Gpu,
    reflection: bool,
    aerosol: f32,
    scale: f32,
    wet: bool,
) -> Vec<f32> {
    let (off_scale, opaque) = arm(gpu, reflection, aerosol, scale, false, wet);
    let (on_scale, radiant) = arm(gpu, reflection, aerosol, scale, true, wet);
    assert_eq!(on_scale.to_bits(), off_scale.to_bits());
    opaque
        .iter()
        .zip(radiant)
        .map(|(off, on)| (on[0] - off[0]) / on_scale)
        .collect()
}

#[cfg(target_os = "macos")]
fn peak(values: &[f32], rows: std::ops::Range<u32>) -> f32 {
    values[(rows.start * EXTENT[0]) as usize..(rows.end * EXTENT[0]) as usize]
        .iter()
        .copied()
        .fold(0.0, f32::max)
}

thrum_test_support::macos_gpu_test! {
fn gather_applies_exposure_and_aerial_transport_once() {
    let Some(gpu) = thrum_test_support::gpu("water optical handoff", Some(wgpu::Backend::Metal)) else { return };
    for reflection in [true, false] {
        let below_horizon = 55..100;
        let expected = isolated(&gpu, reflection, AEROSOL[0], SCALE[0], true);
        let exposed = isolated(&gpu, reflection, AEROSOL[0], SCALE[1], true);
        let hazy = isolated(&gpu, reflection, AEROSOL[1], SCALE[0], true);
        let clear_dry = isolated(&gpu, reflection, AEROSOL[0], SCALE[0], false);
        let hazy_dry = isolated(&gpu, reflection, AEROSOL[1], SCALE[0], false);
        let clear = peak(&expected, below_horizon.clone());
        let exposed = peak(&exposed, below_horizon.clone());
        let hazy = peak(&hazy, below_horizon.clone());
        let dry_rows = if reflection { 34..54 } else { below_horizon };
        let dry = peak(&hazy_dry, dry_rows.clone()) / peak(&clear_dry, dry_rows);
        eprintln!("Metal reflection={reflection} normalized clear={clear} scaled={exposed} hazy={hazy}; lent ratio={dry}, water ratio={}", hazy/clear);
        assert!(clear > 100.0, "a measurable gathered contribution reaches the water");
        assert!((exposed / clear - 1.0).abs() < 0.025, "exposure scale is applied once");
        let tolerance = 0.06;
        assert!(dry > 0.1 && dry < 0.85 && (dry-dry*dry) > 2.0*tolerance,
            "the directly lit reference must separate once from twice");
        assert!((hazy/clear-dry).abs() < tolerance, "camera air applied once");
    }
}
}
