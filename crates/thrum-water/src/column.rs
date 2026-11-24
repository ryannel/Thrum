//! Spectral component records shared by the point query and whitewater.
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub(crate) struct Component {
    pub(super) vector: [f32; 2],
    pub(super) amplitude: f32,
    pub(super) phase: f32,
    pub(super) omega: f32,
    pub(super) inverse_k: f32,
    pub(super) shore_sector: u32,
    pub(super) padding: u32,
}

pub(crate) const COMPONENT_LIMIT: usize = 192;

#[expect(
    clippy::cast_possible_truncation,
    reason = "f32 upper bounds are rounded outwards before upload"
)]
pub(super) fn long_bounds(components: &[thrum_sea_model::Component]) -> [f32; 4] {
    let (height, slope) = components.iter().fold((0.0, 0.0), |(height, slope), wave| {
        let [kx, kz] = wave.wave_vector;
        (
            height + wave.amplitude,
            slope + wave.amplitude * kx.hypot(kz),
        )
    });
    [
        (height as f32).next_up(),
        (slope as f32).next_up(),
        0.0,
        0.0,
    ]
}

impl Component {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPU phases are anchored in f64 before upload"
    )]
    pub(crate) fn at(
        component: &thrum_sea_model::Component,
        origin: [f32; 2],
        seconds: f64,
    ) -> Self {
        let [x, z] = component.wave_vector;
        Self {
            vector: [x as f32, z as f32],
            amplitude: component.amplitude as f32,
            phase: component.anchored_phase(origin.map(f64::from), seconds) as f32,
            omega: component.frequency() as f32,
            inverse_k: x.hypot(z).recip() as f32,
            shore_sector: thrum_water_model::shore_sector(component.wave_vector),
            padding: 0,
        }
    }
}
