//! The wet look's reference: what a forward surface's base colour and roughness become
//! when its pores are wet. Light the ground scatters is partly returned to it at the
//! film's surface, so more of it is absorbed: each channel's albedo `a` becomes
//! `(1 - R)(1 - p) a / (1 - p a)`, with `R` the film's reflectance and `p` the share of
//! the ground's light the film returns to it.

/// Reflectance of the film's surface at normal incidence.
pub const FILM_REFLECTANCE: f64 = 0.02;

/// A forward surface's material, as the wet look reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Linear base colour.
    pub base: [f64; 3],
    /// Perceptual roughness, 0 to 1.
    pub roughness: f64,
    /// Metalness, 0 to 1.
    pub metalness: f64,
    /// The material's own porosity, 0 to 1, where it gives one.
    pub porosity: Option<f64>,
}

/// One channel's albedo under a full film.
pub fn wet_albedo(a: f64, p: f64) -> f64 {
    (1.0 - FILM_REFLECTANCE) * (1.0 - p) * a / (1.0 - p * a)
}

/// Porosity read from roughness and metalness, by Lagarde's rule, where a material gives
/// none: smooth surfaces take no water, rough ones up to all, and metal none.
pub fn porosity(roughness: f64, metalness: f64) -> f64 {
    ((roughness - 0.5) / 0.4).clamp(0.0, 1.0) * (1.0 - metalness.clamp(0.0, 1.0))
}

/// The wet look of a material: its base colour mixed toward its wet albedo by the
/// pores' fill times its porosity, and its roughness toward the film's by half that
/// weight, then toward the film's by `cover`, the surface water's share where no film
/// is drawn over it (0 where one is). `p` is the ground class's share.
pub fn wet_look(
    m: Material,
    pores: f64,
    p: f64,
    film_roughness: f64,
    cover: f64,
) -> ([f64; 3], f64) {
    let porosity = m
        .porosity
        .unwrap_or_else(|| porosity(m.roughness, m.metalness));
    let w = pores.clamp(0.0, 1.0) * porosity.clamp(0.0, 1.0);
    let base = m.base.map(|a| a + (wet_albedo(a, p) - a) * w);
    let r = m.roughness + (film_roughness - m.roughness) * 0.5 * w;
    let r = r + (film_roughness - r) * cover.clamp(0.0, 1.0);
    (base, r)
}
