use crate::{Error, Footprint};
use std::sync::Arc;

/// Identifies a slab in the library stack or the common growth track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeSource {
    /// Index in the library stack.
    Library(u32),
    /// Growth keys are addressed by the object's continuous growth coordinate.
    Growth,
}

/// Game-supplied lattice placement controls, indexed along the cloud type axis.
#[derive(Clone, Debug, PartialEq)]
pub struct HashedShapes {
    /// At most one library instance is selected per square of this many lattice sites.
    pub block_sites: u32,
    /// Proportion of candidate cells that may use a placed shape, by type.
    pub shares: Vec<f64>,
}

/// Authored geometry and its camera-independent footprint. A game retains only
/// this data on a headless host.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedSlab {
    /// Extent of the slab cube in metres.
    pub extent: f64,
    /// Cube's lower face, in metres relative to the condensation plane.
    pub floor: f64,
    /// Authored downwind heading in the X/Z plane, radians.
    pub heading: f64,
    /// Source footprint in metres relative to the condensation plane.
    pub footprint: Arc<Footprint>,
}

/// Ordered footprints corresponding to the two stacks a game cooks.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedLibrary {
    shapes: Vec<PlacedSlab>,
    growth: Vec<PlacedSlab>,
}

impl PlacedLibrary {
    /// Validate the geometry and admit two independently addressed stacks.
    ///
    /// # Errors
    /// Rejects an empty library, inconsistent slab resolution, non-cubic
    /// layouts, and growth keys with different physical frames.
    pub fn new(shapes: Vec<PlacedSlab>, growth: Vec<PlacedSlab>) -> Result<Self, Error> {
        let Some(first) = shapes.first() else {
            return Err(Error::Invalid);
        };
        let (size, _) = first.footprint.layout();
        let same_frame = |a: &PlacedSlab, b: &PlacedSlab| {
            [a.extent, a.floor, a.heading].map(f64::to_bits)
                == [b.extent, b.floor, b.heading].map(f64::to_bits)
        };
        if size[0] != size[1]
            || !size[0].is_power_of_two()
            || shapes.iter().chain(&growth).any(|slab| {
                let (layout, spacing) = slab.footprint.layout();
                layout != size
                    || !slab.extent.is_finite()
                    || !slab.floor.is_finite()
                    || !slab.heading.is_finite()
                    || (slab.extent / (size[0] as f64) - spacing).abs() > spacing * 1e-6
            })
            || growth
                .windows(2)
                .any(|pair| !same_frame(&pair[0], &pair[1]))
        {
            return Err(Error::Invalid);
        }
        Ok(Self { shapes, growth })
    }

    /// Static shapes in their cooked stack order.
    pub fn shapes(&self) -> &[PlacedSlab] {
        &self.shapes
    }

    /// Equally timed keys in their cooked stack order.
    pub fn growth(&self) -> &[PlacedSlab] {
        &self.growth
    }

    /// Slab geometry shared by both keys of a placed shape.
    pub fn geometry(&self, source: ShapeSource) -> Option<&PlacedSlab> {
        match source {
            ShapeSource::Library(index) => self.shapes.get(index as usize),
            ShapeSource::Growth => self.growth.first(),
        }
    }

    /// Index of each sampled slab and the second slab's weight.
    ///
    /// # Panics
    /// Only if the key count exceeds the addressable storage of a volume product.
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "clamped growth and key count bound the slab index"
    )]
    pub fn keys(&self, source: ShapeSource, growth: f64) -> ([u32; 2], f64) {
        if let ShapeSource::Library(index) = source {
            return ([index; 2], 0.0);
        }
        let end = u32::try_from(self.growth.len().saturating_sub(1)).expect("volume stack size");
        let key = growth.clamp(0.0, 1.0) * f64::from(end);
        let lower = key as u32;
        ([lower, (lower + 1).min(end)], key.fract())
    }

    pub(crate) fn column(&self, source: ShapeSource, growth: f64, local: [f64; 2]) -> [f64; 3] {
        let ShapeSource::Growth = source else {
            return self
                .geometry(source)
                .map_or([0.0; 3], |slab| slab.footprint.column(local));
        };
        let ([a, b], blend) = self.keys(source, growth);
        let first = self.growth[a as usize].footprint.column(local);
        if blend == 0.0 {
            return first;
        }
        let second = self.growth[b as usize].footprint.column(local);
        let integral = first[0] * (1.0 - blend) + second[0] * blend;
        if first[0] == 0.0 {
            [integral, second[1], second[2]]
        } else if second[0] == 0.0 {
            [integral, first[1], first[2]]
        } else {
            [integral, first[1].min(second[1]), first[2].max(second[2])]
        }
    }
}
