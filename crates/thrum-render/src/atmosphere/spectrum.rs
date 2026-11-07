//! Fixed eight-node Earth profile. The equal observer-mass bins integrate a
//! 5778 K Planck source against the analytic CIE fit of Wyman et al. (2013),
//! https://jcgt.org/published/0002/02/01/. Rayleigh uses lambda^-4. Ozone uses a
//! smooth band through the existing 440, 550 and 680 nm anchors.

use std::fmt::Write;

use glam::Vec3;

pub(super) const LUMA: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);
pub(super) struct Profile {
    pub(super) rayleigh: [f32; 8],
    pub(super) ozone: [f32; 8],
    pub(super) rgb: [[f32; 8]; 3],
    pub(super) mie_extinction: f32,
    pub(super) mie_scattering: f32,
}

pub(super) const EARTH: Profile = Profile {
    mie_extinction: 4.4e-6,
    mie_scattering: 3.996e-6,
    // 428.84, 450.21, 471.18, 514.23, 550.21, 574.75, 597.70, 630.96 nm.
    rayleigh: [
        3.668_017e-5,
        3.019_671_6e-5,
        2.516_967_1e-5,
        1.774_173_7e-5,
        1.353_632_5e-5,
        1.136_824_6e-5,
        9.719_996e-6,
        7.827_13e-6,
    ],
    ozone: [
        5.059_233e-8,
        1.321_905_6e-7,
        2.965_948_7e-7,
        1.026_923_3e-6,
        1.885_528e-6,
        2.279_152e-6,
        2.307_383_5e-6,
        1.770_035e-6,
    ],
    // Linear Rec.709 weights per unit source lux. A luminance-preserving white
    // offset maps vacuum to (1,1,1), retaining the authored source convention.
    rgb: [
        [
            0.046_199_19,
            0.026_971_36,
            -0.058_030_62,
            -0.373_742_67,
            -0.058_011_713,
            0.257_310_87,
            0.498_560_07,
            0.660_743_53,
        ],
        [
            -0.042_857_476,
            -0.032_444_324,
            0.026_634_296,
            0.462_855_55,
            0.378_189_9,
            0.194_632_63,
            0.054_261_465,
            -0.041_272_02,
        ],
        [
            0.327_534_9,
            0.326_863_88,
            0.316_224_96,
            0.064_721_52,
            -0.020_442_596,
            -0.012_335_314,
            -0.004_137_213,
            0.001_569_856_5,
        ],
    ],
};

impl Profile {
    pub(super) fn project(&self, values: [f32; 8]) -> [f32; 3] {
        let rgb = Vec3::from_array(
            self.rgb
                .map(|weights| weights.into_iter().zip(values).map(|(w, x)| w * x).sum()),
        );
        let low = rgb.min_element();
        if low >= 0.0 {
            return rgb.to_array();
        }
        let y = rgb.dot(LUMA).max(0.0);
        Vec3::splat(y)
            .lerp(rgb, y / (y - low))
            .max(Vec3::ZERO)
            .to_array()
    }

    pub(super) fn shader(&self, body: &str) -> String {
        let mut source = String::with_capacity(body.len() + 1024);
        for (name, values) in [
            ("RAYLEIGH_BETA", self.rayleigh),
            ("OZONE_BETA", self.ozone),
            ("RGB_RED", self.rgb[0]),
            ("RGB_GREEN", self.rgb[1]),
            ("RGB_BLUE", self.rgb[2]),
        ] {
            let [a, b, c, d, e, f, g, h] = values;
            writeln!(source, "const {name} = array(vec4<f32>({a:e},{b:e},{c:e},{d:e}),vec4<f32>({e:e},{f:e},{g:e},{h:e}));")
                .expect("formatting into String cannot fail");
        }
        let [r, g, b] = LUMA.to_array();
        writeln!(source, "const SPECTRUM_LUMA = vec3<f32>({r},{g},{b});\nconst MIE_EXTINCTION = {:e};\nconst MIE_SCATTERING = {:e};", self.mie_extinction, self.mie_scattering)
            .expect("formatting into String cannot fail");
        source.push_str(body);
        source
    }
}
