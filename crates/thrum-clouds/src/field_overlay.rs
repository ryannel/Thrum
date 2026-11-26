/// A presentation-only view of a cloud field. Off displays the ordinary sky.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum FieldOverlay {
    /// Ordinary sky.
    #[default]
    Off,
    /// Weather-map coverage.
    Coverage,
    /// Weather-map type.
    Type,
    /// Weather-map cloud top.
    Top,
    /// Weather-map lifecycle age.
    Age,
    /// Placed cloud slots.
    PlacedSlots,
    /// Optical depth toward the Sun on the ground.
    SunPath,
    /// Visible-layer refresh age.
    LayerAge,
}

impl FieldOverlay {
    const ALL: [Self; 8] = [
        Self::Off,
        Self::Coverage,
        Self::Type,
        Self::Top,
        Self::Age,
        Self::PlacedSlots,
        Self::SunPath,
        Self::LayerAge,
    ];

    /// Display labels in the order of the integer control stops.
    pub const LABELS: &'static [&'static str] = &[
        "Off",
        "Coverage",
        "Cloud type",
        "Cloud top",
        "Cloud age",
        "Placed slots",
        "Sun-path optical depth",
        "Layer refresh age",
    ];

    /// Select a discrete control stop. Invalid values select the ordinary sky.
    #[must_use]
    pub fn from_stop(stop: f32) -> Self {
        Self::ALL
            .into_iter()
            .zip(0_u16..)
            .find_map(|(overlay, index)| (f32::from(index) == stop).then_some(overlay))
            .unwrap_or_default()
    }
}
