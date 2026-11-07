//! Small checked products used by a running game without authoring storage.

const VALUE_COUNT: usize = 14;

/// Checked photometric source quantities retained without display exposure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentValues {
    numbers: [f64; VALUE_COUNT],
}

impl EnvironmentValues {
    /// Construct values only when the authored numeric contract is satisfied.
    pub fn from_numbers(numbers: [f64; VALUE_COUNT]) -> Option<Self> {
        // Match the source parser's accepted tolerance and association.
        let length = numbers[0].hypot(numbers[1]).hypot(numbers[2]);
        let luminance = 0.2126 * numbers[4] + 0.7152 * numbers[5] + 0.0722 * numbers[6];
        let valid = numbers.iter().all(|x| x.is_finite())
            && (length - 1.0).abs() <= 1e-9
            && (luminance - 1.0).abs() <= 1e-9
            && numbers[3..].iter().all(|x| *x >= 0.0);
        valid.then_some(Self { numbers })
    }

    /// Fixed-layout quantities used by the product encoder.
    pub fn numbers(self) -> [f64; VALUE_COUNT] {
        self.numbers
    }

    /// Geometric world direction from receiver toward the Sun.
    pub fn toward_sun(self) -> [f64; 3] {
        [self.numbers[0], self.numbers[1], self.numbers[2]]
    }

    /// Raw perpendicular Sun illuminance in lux.
    pub fn sun_lux(self) -> f64 {
        self.numbers[3]
    }

    /// Linear Rec.709 colour normalized to unit photometric luminance.
    pub fn sun_colour(self) -> [f64; 3] {
        [self.numbers[4], self.numbers[5], self.numbers[6]]
    }

    /// Geometric Sun angular radius in radians.
    pub fn sun_angular_radius(self) -> f64 {
        self.numbers[7]
    }

    /// Isotropic hemispherical ambient irradiance in lux.
    pub fn ambient_lux(self) -> [f64; 3] {
        [self.numbers[8], self.numbers[9], self.numbers[10]]
    }

    /// Independent linear Rec.709 background radiance in cd/m².
    pub fn background_radiance(self) -> [f64; 3] {
        [self.numbers[11], self.numbers[12], self.numbers[13]]
    }
}

/// Direct-input product identity; unrelated accepted files do not change it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnvironmentProductRevision([u8; 32]);

impl EnvironmentProductRevision {
    /// Construct a revision already verified by the content producer.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Raw digest for a trusted product manifest or visual identity.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Source-free environment payload for a running game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentProduct {
    revision: EnvironmentProductRevision,
    values: EnvironmentValues,
}

impl EnvironmentProduct {
    /// Assemble a runtime product from checked values and a verified revision.
    pub fn new(revision: EnvironmentProductRevision, values: EnvironmentValues) -> Self {
        Self { revision, values }
    }

    /// Direct-input product identity.
    pub fn revision(self) -> EnvironmentProductRevision {
        self.revision
    }

    /// Checked photometric quantities.
    pub fn values(self) -> EnvironmentValues {
        self.values
    }
}

#[cfg(test)]
mod tests {
    use super::{EnvironmentProduct, EnvironmentProductRevision, EnvironmentValues};

    #[test]
    fn checked_product_rejects_bad_light_and_keeps_revision() {
        let mut numbers = [0.0; 14];
        numbers[1] = 1.0;
        numbers[4..7].fill(1.0);
        let values = EnvironmentValues::from_numbers(numbers).expect("valid unit light");
        let revision = EnvironmentProductRevision::from_bytes([7; 32]);
        let product = EnvironmentProduct::new(revision, values);
        assert_eq!(product.revision().as_bytes(), &[7; 32]);
        assert_eq!(product.values().toward_sun(), [0.0, 1.0, 0.0]);

        numbers[3] = f64::NAN;
        assert!(EnvironmentValues::from_numbers(numbers).is_none());
        numbers[3] = -1.0;
        assert!(EnvironmentValues::from_numbers(numbers).is_none());
    }
}
