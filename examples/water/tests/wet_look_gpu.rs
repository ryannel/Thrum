//! Set the same wetness provider on an entity material and cooked placements.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn both_forward_material_paths_darkening() {
    let name = "both_forward_material_paths_darkening";
    let Some(gpu) = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal)) else {
        return;
    };
    support::island::with_cooked_island(name, || check(&gpu));
}
}

#[cfg(target_os = "macos")]
fn check(gpu: &thrum_test_support::Gpu) {
    use std::sync::{Arc, Mutex};
    use thrum_app::sky::{Camera, Controls, CookedMaterial, MeshPlacement, SkyGame};
    use thrum_water::wet_ground::{GroundClass, WetGround};
    use thrum_water_model::CanopyGrid;

    let camera = Camera::look_at([0.0, 2.8, 4.6], [0.0, 0.1, 0.0], 44.0);
    let classes = [GroundClass {
        relief: 0.001,
        steepest: 1.0,
        share: 0.67,
        porosity: 1.0,
        film_roughness: 0.08,
        ..GroundClass::default()
    }];
    for (albedo, metalness, sheltered) in [
        (0.1, 0.0, false),
        (0.4, 0.0, false),
        (0.8, 0.0, false),
        (0.4, 1.0, false),
        (0.4, 0.0, true),
    ] {
        let canopy = sheltered.then(|| CanopyGrid {
            origin: [-8.0; 2],
            cell: 4.0,
            size: [4, 4],
            edge_width: 0.3,
            columns: vec![[[5.0, 0.0], [-1e20, 1.0]]; 16],
        });
        let mut field = WetGround::new(canopy, None, &classes, 0).expect("wetness table");
        let game = SkyGame::new(Controls::new(), camera, |_, sky| {
            sky.sun_elevation_degrees = 53.0;
        })
        .with_ground(Some([albedo; 3]))
        .with_wetness(field.provider())
        .with_meshes(vec![
            MeshPlacement::new("meshes/rock", "textures/white")
                .at([0.0, 0.5, 0.0])
                .scale(1.25)
                .material(CookedMaterial {
                    base_colour: [albedo, albedo, albedo, 1.0],
                    roughness: 0.88,
                    metalness,
                }),
        ])
        .expect("cooked rock material");
        let mut f = support::Fixture::from_game(gpu, [112, 96], game, Arc::new(Mutex::default()));
        f.warm_cooked();
        f.frame_without_timings(1.0);
        let dry = f.image();
        assert!(field.publish(&classes, &[[[0.0, 1.0]; 3]], &[]));
        f.frame_without_timings(1.01);
        let wet = f.image();
        let brightness = |pixels: &[u8], x: usize, y: usize| {
            let i = 4 * (112 * y + x);
            pixels[i..i + 3].iter().map(|&v| u32::from(v)).sum::<u32>()
        };
        let compare = |x0, x1, y0, y1| {
            let coords = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y)));
            coords.fold((0, 0), |(dry_total, wet_total), (x, y)| {
                (
                    dry_total + brightness(&dry, x, y),
                    wet_total + brightness(&wet, x, y),
                )
            })
        };
        let (dry_rock, wet_rock) = compare(50, 62, 40, 45);
        let (dry_floor, wet_floor) = compare(40, 72, 77, 84);
        if sheltered {
            assert_eq!(dry_rock, wet_rock, "roof covers the cooked material");
            assert_eq!(dry_floor, wet_floor, "roof covers the entity material");
        } else {
            assert!(
                wet_floor < dry_floor,
                "entity surface {albedo} darkens ({wet_floor} vs {dry_floor})"
            );
            if metalness > 0.0 {
                assert_eq!(wet_rock, dry_rock, "metal is unchanged");
            } else {
                assert!(
                    wet_rock < dry_rock,
                    "cooked matte {albedo} darkens ({wet_rock} vs {dry_rock})"
                );
            }
        }
        if (albedo - 0.4).abs() < f32::EPSILON && metalness == 0.0 && !sheltered {
            let mut rough_control = classes;
            rough_control[0].film_roughness = 1.0;
            assert!(field.publish(&rough_control, &[[[0.0, 1.0]; 3]], &[]));
            f.frame_without_timings(1.02);
            let varnish_disabled = f.image();
            assert_ne!(
                brightness(&wet, 56, 42),
                brightness(&varnish_disabled, 56, 42),
                "the cooked surface should respond to reduced roughness independently of albedo"
            );
            assert!(
                wet.chunks_exact(4)
                    .zip(varnish_disabled.chunks_exact(4))
                    .enumerate()
                    .filter(|(i, _)| i / 112 >= 77)
                    .any(|(_, (a, b))| a != b),
                "the entity surface should respond to reduced roughness independently of albedo"
            );
        }
    }
}
