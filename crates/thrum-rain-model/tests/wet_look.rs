//! M2.4: the wet look's reference. Each channel darkens by `(1 - R)(1 - p) a / (1 - p a)`,
//! mixed in by the pores' fill times porosity; the test works the ratios from that form
//! itself and holds them to part 4's figures.
use thrum_rain_model::{FILM_REFLECTANCE, Material, wet_look};

fn ratio(a: f64, p: f64) -> f64 {
    (1.0 - 0.02) * (1.0 - p) / (1.0 - p * a)
}

fn saturation(c: [f64; 3]) -> f64 {
    let (hi, lo) = (
        c.iter().copied().fold(0.0, f64::max),
        c.iter().copied().fold(1.0, f64::min),
    );
    (hi - lo) / hi
}

fn porous(base: [f64; 3]) -> Material {
    Material {
        base,
        roughness: 0.8,
        metalness: 0.0,
        porosity: Some(1.0),
    }
}

#[test]
fn dark_ground_darkens_more_than_pale_and_colours_deepen() {
    assert_eq!(FILM_REFLECTANCE, 0.02);
    let (wet, _) = wet_look(porous([0.1, 0.8, 0.4]), 1.0, 0.47, 0.05, 0.0);
    for (i, (a, want)) in [(0.1, 0.55), (0.8, 0.83)].into_iter().enumerate() {
        let got = wet[i] / a;
        assert!((got - want).abs() <= 0.01, "albedo {a}: {got}");
        assert!((got - ratio(a, 0.47)).abs() < 1e-12);
    }
    // Mixed in by the pores' fill times porosity.
    let half = |fill: f64, porosity: f64| {
        let m = Material {
            porosity: Some(porosity),
            ..porous([0.3, 0.3, 0.3])
        };
        wet_look(m, fill, 0.47, 0.05, 0.0).0[0]
    };
    let full = 0.3 * ratio(0.3, 0.47);
    for (fill, porosity) in [(0.5, 1.0), (1.0, 0.5), (0.25, 0.8)] {
        let w = fill * porosity;
        assert!((half(fill, porosity) - (0.3 + (full - 0.3) * w)).abs() < 1e-12);
    }
    assert_eq!(half(0.0, 1.0), 0.3);
    // A brown deepens.
    let brown = [0.30, 0.22, 0.14];
    for (p, want) in [(0.47, 0.57), (0.7, 0.59)] {
        let (wet, _) = wet_look(porous(brown), 1.0, p, 0.05, 0.0);
        let s = saturation(wet);
        println!(
            "brown at p {p}: saturation {:.3} -> {s:.3}",
            saturation(brown)
        );
        assert!(s > saturation(brown) && (s - want).abs() < 0.01);
    }
}

#[test]
fn metal_keeps_its_colour_and_porosity_follows_roughness_where_a_material_gives_none() {
    let metal = Material {
        base: [0.9, 0.6, 0.3],
        roughness: 0.9,
        metalness: 1.0,
        porosity: None,
    };
    let (wet, rough) = wet_look(metal, 1.0, 0.47, 0.05, 0.0);
    assert_eq!(wet, metal.base);
    assert_eq!(rough, metal.roughness);
    // Lagarde's rule, worked here: none below a roughness of 0.5, all from 0.9, none for
    // metal.
    for (roughness, metalness, porosity) in [
        (0.3, 0.0, 0.0),
        (0.7, 0.0, 0.5),
        (0.95, 0.0, 1.0),
        (0.7, 0.5, 0.25),
    ] {
        let m = Material {
            base: [0.4; 3],
            roughness,
            metalness,
            porosity: None,
        };
        let (wet, r) = wet_look(m, 1.0, 0.47, 0.05, 0.0);
        let want = 0.4 + (0.4 * ratio(0.4, 0.47) - 0.4) * porosity;
        assert!(
            (wet[0] - want).abs() < 1e-12,
            "roughness {roughness}, metalness {metalness}"
        );
        // Roughness falls toward the film's by half the weight.
        assert!((r - (roughness + (0.05 - roughness) * 0.5 * porosity)).abs() < 1e-12);
    }
    // Where no film is drawn, the surface water's share lowers the roughness too.
    let (_, r) = wet_look(metal, 0.0, 0.47, 0.05, 0.5);
    assert!((r - (0.9 + (0.05 - 0.9) * 0.5)).abs() < 1e-12);
}
