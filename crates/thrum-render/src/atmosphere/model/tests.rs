use super::*;

#[test]
fn sky_mapping_matches_planet_horizon_and_angular_spans() {
    for height in [0.0_f32, 2.0, 100_000.0] {
        let [mu, elevation, sky_span, ground_span] = sky_mapping(height);
        let h = f64::from(height);
        let radius = f64::from(RADIUS);
        let expected_mu = -(h * (2.0 * radius + h)).sqrt() / (radius + h);
        let expected_elevation = expected_mu.asin();
        assert!((f64::from(mu) - expected_mu).abs() < 1e-7);
        assert!((f64::from(elevation) - expected_elevation).abs() < 1e-7);
        assert!(
            (f64::from(sky_span) - (std::f64::consts::FRAC_PI_2 - expected_elevation)).abs() < 2e-7
        );
        assert!(
            (f64::from(ground_span) - (std::f64::consts::FRAC_PI_2 + expected_elevation)).abs()
                < 2e-7
        );
        assert!(sky_span > 0.0 && ground_span > 0.0);
        assert!((sky_span + ground_span - std::f32::consts::PI).abs() < 5e-7);
    }
    assert_eq!(
        sky_mapping(0.0),
        [
            -0.0,
            -0.0,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::FRAC_PI_2
        ]
    );
}

#[test]
fn shipping_atmosphere_shader_layouts_keep_mapping_and_view_aux_separate() {
    let common = concat!(
        include_str!("../storage.wgsl"),
        "\n",
        concat!(
            include_str!("../geometry.wgsl"),
            "\n",
            include_str!("../common.wgsl")
        )
    );
    let spectral = concat!(
        include_str!("../transport.wgsl"),
        "\n",
        include_str!("../night.wgsl")
    );
    for source in [
        EARTH.shader(&format!(
            "{common}\n{spectral}\n{}\n{}",
            include_str!("../lut.wgsl"),
            include_str!("../ground.wgsl")
        )),
        format!("{common}\n{}", include_str!("../sky.wgsl")),
        format!(
            "{common}\n{}\n{}",
            include_str!("../irradiance.wgsl"),
            include_str!("../diffuse.wgsl")
        ),
        EARTH.shader(&format!(
            "{common}\n{spectral}\n{}",
            include_str!("../aerial.wgsl")
        )),
        format!(
            "{}\n{}\n{}\n{}\n{}",
            concat!(
                include_str!("../storage.wgsl"),
                "\n",
                include_str!("../brdf.wgsl")
            ),
            include_str!("../irradiance.wgsl"),
            concat!(
                include_str!("../../synthetic.wgsl"),
                "\n",
                include_str!("../../synthetic/horizon_taps.wgsl")
            ),
            concat!(
                include_str!("../../synthetic_aerial.wgsl"),
                "\n",
                include_str!("../../synthetic_aerial_clear.wgsl")
            ),
            concat!(
                include_str!("../geometry.wgsl"),
                "\n",
                include_str!("../../synthetic_sun.wgsl"),
                "\n",
                include_str!("../../synthetic_environment.wgsl")
            )
        ),
        format!(
            "{}\n{}\n{}\n{}\n{}",
            concat!(
                include_str!("../storage.wgsl"),
                "\n",
                include_str!("../brdf.wgsl")
            ),
            include_str!("../irradiance.wgsl"),
            concat!(
                include_str!("../../synthetic.wgsl"),
                "\n",
                include_str!("../../synthetic/horizon_taps.wgsl")
            ),
            concat!(
                include_str!("../../synthetic_aerial.wgsl"),
                "\n",
                include_str!("../../synthetic_aerial_shadowed.wgsl")
            ),
            concat!(
                include_str!("../geometry.wgsl"),
                "\n",
                include_str!("../../synthetic_sun.wgsl"),
                "\n",
                include_str!("../../synthetic_environment.wgsl")
            )
        ),
    ] {
        let module = wgpu::naga::front::wgsl::parse_str(&source).unwrap();
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        for (_, ty) in module.types.iter() {
            if matches!(ty.name.as_deref(), Some("Atmosphere" | "AerialFrame")) {
                let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
                    panic!("uniform struct");
                };
                assert_eq!(*span, 128);
                assert_eq!(members[4].name.as_deref(), Some("sky_mapping"));
                assert_eq!(members[4].offset, 112);
            }
            if ty.name.as_deref() == Some("ViewRecord") {
                let wgpu::naga::TypeInner::Struct { span, .. } = ty.inner else {
                    panic!("view struct");
                };
                assert_eq!(span, 160);
            }
        }
    }
}
