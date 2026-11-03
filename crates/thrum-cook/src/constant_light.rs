//! Pure constant-light normalization and the selected 160-byte CPU product ABI.
//!
//! Callers own F3 schema validation, source/light closure and admitted-pin access.
//! These types carry no source authentication, cook key, manifest or build binding.
//! Floating-point byte identity is scoped to a pinned compiler/target/mode class.

use std::fmt;

const MAGIC: &[u8; 8] = b"THRME1C\0";
const FORMAT_VERSION: u16 = 1;
const UNSUPPORTED_MASK: u32 = 0x7f;
const NORMALIZATION_TOLERANCE: f64 = 1e-12;
const ENVIRONMENT_OFFSET: usize = 16;
const LIGHT_OFFSET: usize = 32;
const DIRECTION_OFFSET: usize = 48;
const COLOUR_OFFSET: usize = 72;
const ILLUMINANCE_OFFSET: usize = 96;
const RADIUS_OFFSET: usize = 104;
const AMBIENT_OFFSET: usize = 112;
const BACKGROUND_OFFSET: usize = 136;

/// Known numeric and identity values from an F3-validated `constant-light-v1`
/// source and its selected light. Opaque fields and labels do not belong here.
///
/// Constructing this value does not validate or authenticate source content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstantLightInputV1 {
    /// Environment UUID octets in displayed hexadecimal order, without hyphens.
    pub environment_id: [u8; 16],
    /// Selected light UUID octets in the same order.
    pub light_id: [u8; 16],
    /// Geometric world-space receiver-to-source direction, X/Y/Z.
    pub toward_source_unit: [f64; 3],
    /// Nonnegative linear Rec.709/D65 photometric colour with source `Y≈1`.
    pub unit_luminance_linear_rec709: [f64; 3],
    /// Raw perpendicular Sun illuminance in lux, with no applied attenuation.
    pub perpendicular_illuminance_lux: f64,
    /// Authored geometric angular radius in radians.
    pub angular_radius_rad: f64,
    /// Isotropic incident diffuse irradiance in RGB lux.
    pub ambient_irradiance_lux: [f64; 3],
    /// Independent terminal background radiance in RGB cd/m².
    pub background_radiance_cd_m2: [f64; 3],
}

/// Immutable normalized constant-light CPU payload, constant over space/time.
///
/// Wind, cloud tau, precipitation, air, thermal, Moon and air quality are
/// unsupported. This payload alone cannot authorize a runnable build.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstantLightProductV1 {
    values: ConstantLightInputV1,
}

impl ConstantLightProductV1 {
    /// Exact serialized payload size, independent of Rust struct layout.
    pub const BYTE_LEN: usize = 160;

    /// Normalize already F3-validated values using the selected ordered f64
    /// expressions and canonicalize every zero to positive zero.
    ///
    /// This performs numeric/identity safety checks, not schema validation or
    /// source admission. The caller must establish the selected source/light
    /// binding and schema-1 direction/colour source tolerances first.
    ///
    /// # Errors
    /// Refuses invalid identities, nonfinite/negative quantities or a result
    /// outside the cooked `1e-12` unit-length/`Y=1` residual bounds.
    pub fn cook(mut input: ConstantLightInputV1) -> Result<Self, ConstantLightError> {
        validate_quantities(&input)?;
        let length = ordered_length(input.toward_source_unit);
        let luminance = ordered_luminance(input.unit_luminance_linear_rec709);
        for value in &mut input.toward_source_unit {
            *value /= length;
        }
        for value in &mut input.unit_luminance_linear_rec709 {
            *value /= luminance;
        }
        for value in numeric_values_mut(&mut input) {
            if *value == 0.0 {
                *value = 0.0;
            }
        }
        let product = Self { values: input };
        product.validate()?;
        Ok(product)
    }

    /// The cooked values. Direction and colour satisfy the product residuals;
    /// all zeros have the positive-zero IEEE-754 representation.
    pub fn values(&self) -> &ConstantLightInputV1 {
        &self.values
    }

    /// Explicit fixed little-endian encoding. Header flags are zero and the
    /// unsupported mask is exactly `0x7f`.
    pub fn to_bytes(&self) -> [u8; Self::BYTE_LEN] {
        let mut bytes = [0; Self::BYTE_LEN];
        bytes[..8].copy_from_slice(MAGIC);
        bytes[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes[12..16].copy_from_slice(&UNSUPPORTED_MASK.to_le_bytes());
        bytes[ENVIRONMENT_OFFSET..LIGHT_OFFSET].copy_from_slice(&self.values.environment_id);
        bytes[LIGHT_OFFSET..DIRECTION_OFFSET].copy_from_slice(&self.values.light_id);
        write_triplet(&mut bytes, DIRECTION_OFFSET, self.values.toward_source_unit);
        write_triplet(
            &mut bytes,
            COLOUR_OFFSET,
            self.values.unit_luminance_linear_rec709,
        );
        write_f64(
            &mut bytes,
            ILLUMINANCE_OFFSET,
            self.values.perpendicular_illuminance_lux,
        );
        write_f64(&mut bytes, RADIUS_OFFSET, self.values.angular_radius_rad);
        write_triplet(
            &mut bytes,
            AMBIENT_OFFSET,
            self.values.ambient_irradiance_lux,
        );
        write_triplet(
            &mut bytes,
            BACKGROUND_OFFSET,
            self.values.background_radiance_cd_m2,
        );
        bytes
    }

    /// Strictly decode a product without repairing or renormalizing its bytes.
    ///
    /// # Errors
    /// Refuses any layout mismatch, invalid UUID, nonfinite/negative quantity,
    /// negative zero or failed cooked normalization residual. External digest
    /// and expected-identity verification remain the consuming manifest's job.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ConstantLightError> {
        if bytes.len() != Self::BYTE_LEN {
            return Err(ConstantLightError::Length);
        }
        if &bytes[..8] != MAGIC {
            return Err(ConstantLightError::Magic);
        }
        if bytes[8..10] != FORMAT_VERSION.to_le_bytes() {
            return Err(ConstantLightError::Version);
        }
        if bytes[10..12] != [0, 0] {
            return Err(ConstantLightError::Flags);
        }
        if bytes[12..16] != UNSUPPORTED_MASK.to_le_bytes() {
            return Err(ConstantLightError::UnsupportedMask);
        }
        let product = Self {
            values: ConstantLightInputV1 {
                environment_id: bytes[ENVIRONMENT_OFFSET..LIGHT_OFFSET]
                    .try_into()
                    .expect("checked fixed product length"),
                light_id: bytes[LIGHT_OFFSET..DIRECTION_OFFSET]
                    .try_into()
                    .expect("checked fixed product length"),
                toward_source_unit: read_triplet(bytes, DIRECTION_OFFSET),
                unit_luminance_linear_rec709: read_triplet(bytes, COLOUR_OFFSET),
                perpendicular_illuminance_lux: read_f64(bytes, ILLUMINANCE_OFFSET),
                angular_radius_rad: read_f64(bytes, RADIUS_OFFSET),
                ambient_irradiance_lux: read_triplet(bytes, AMBIENT_OFFSET),
                background_radiance_cd_m2: read_triplet(bytes, BACKGROUND_OFFSET),
            },
        };
        product.validate()?;
        Ok(product)
    }

    fn validate(&self) -> Result<(), ConstantLightError> {
        validate_quantities(&self.values)?;
        if numeric_values(&self.values).any(|value| value == 0.0 && value.is_sign_negative()) {
            return Err(ConstantLightError::NegativeZero);
        }
        if (ordered_length(self.values.toward_source_unit) - 1.0).abs() > NORMALIZATION_TOLERANCE
            || (ordered_luminance(self.values.unit_luminance_linear_rec709) - 1.0).abs()
                > NORMALIZATION_TOLERANCE
        {
            return Err(ConstantLightError::Normalization);
        }
        Ok(())
    }
}

/// A refused constant-light cook or payload decode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstantLightError {
    /// Payload is not exactly 160 bytes.
    Length,
    /// Product magic differs.
    Magic,
    /// Product format version differs.
    Version,
    /// Reserved flags are nonzero.
    Flags,
    /// Unsupported quantities are not exactly the seven selected quantities.
    UnsupportedMask,
    /// A UUID has an invalid version or RFC-4122 variant.
    Uuid,
    /// Environment and light IDs collide in the declared record namespace.
    IdentityCollision,
    /// A numeric quantity is NaN or infinite.
    Nonfinite,
    /// A colour, illuminance, radius, ambient or background channel is negative.
    NegativeQuantity,
    /// Encoded zero has its sign bit set.
    NegativeZero,
    /// Cooked direction or colour exceeds the selected residual tolerance.
    Normalization,
}

impl fmt::Display for ConstantLightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Length => "constant-light payload must be exactly 160 bytes",
            Self::Magic => "invalid constant-light product magic",
            Self::Version => "unsupported constant-light product version",
            Self::Flags => "nonzero constant-light reserved flags",
            Self::UnsupportedMask => "invalid constant-light unsupported mask",
            Self::Uuid => "invalid constant-light UUID",
            Self::IdentityCollision => "environment and light UUIDs collide",
            Self::Nonfinite => "nonfinite constant-light quantity",
            Self::NegativeQuantity => "negative constant-light quantity",
            Self::NegativeZero => "noncanonical constant-light negative zero",
            Self::Normalization => "invalid constant-light normalization residual",
        })
    }
}

impl std::error::Error for ConstantLightError {}

fn validate_quantities(values: &ConstantLightInputV1) -> Result<(), ConstantLightError> {
    // Match F3's canonical RFC-4122-form UUID policy, including versions 1–8.
    for id in [values.environment_id, values.light_id] {
        if !(1..=8).contains(&(id[6] >> 4)) || id[8] & 0xc0 != 0x80 {
            return Err(ConstantLightError::Uuid);
        }
    }
    if values.environment_id == values.light_id {
        return Err(ConstantLightError::IdentityCollision);
    }
    if numeric_values(values).any(|value| !value.is_finite()) {
        return Err(ConstantLightError::Nonfinite);
    }
    if numeric_values(values).skip(3).any(|value| value < 0.0) {
        return Err(ConstantLightError::NegativeQuantity);
    }
    Ok(())
}

fn numeric_values(values: &ConstantLightInputV1) -> impl Iterator<Item = f64> + '_ {
    values
        .toward_source_unit
        .iter()
        .chain(&values.unit_luminance_linear_rec709)
        .chain(std::iter::once(&values.perpendicular_illuminance_lux))
        .chain(std::iter::once(&values.angular_radius_rad))
        .chain(&values.ambient_irradiance_lux)
        .chain(&values.background_radiance_cd_m2)
        .copied()
}

fn numeric_values_mut(values: &mut ConstantLightInputV1) -> impl Iterator<Item = &mut f64> {
    values
        .toward_source_unit
        .iter_mut()
        .chain(&mut values.unit_luminance_linear_rec709)
        .chain(std::iter::once(&mut values.perpendicular_illuminance_lux))
        .chain(std::iter::once(&mut values.angular_radius_rad))
        .chain(&mut values.ambient_irradiance_lux)
        .chain(&mut values.background_radiance_cd_m2)
}

fn ordered_length([x, y, z]: [f64; 3]) -> f64 {
    (((x * x) + (y * y)) + (z * z)).sqrt()
}

fn ordered_luminance([r, g, b]: [f64; 3]) -> f64 {
    ((0.2126 * r) + (0.7152 * g)) + (0.0722 * b)
}

fn write_f64(bytes: &mut [u8; 160], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_triplet(bytes: &mut [u8; 160], offset: usize, values: [f64; 3]) {
    for (index, value) in values.into_iter().enumerate() {
        write_f64(bytes, offset + index * 8, value);
    }
}

fn read_f64(bytes: &[u8], offset: usize) -> f64 {
    f64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("checked fixed product length"),
    )
}

fn read_triplet(bytes: &[u8], offset: usize) -> [f64; 3] {
    std::array::from_fn(|index| read_f64(bytes, offset + index * 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENVIRONMENT: [u8; 16] = [0x10, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
    const LIGHT: [u8; 16] = [0x10, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 2];

    fn source() -> ConstantLightInputV1 {
        ConstantLightInputV1 {
            environment_id: ENVIRONMENT,
            light_id: LIGHT,
            toward_source_unit: [0.0, 1.0, 0.0],
            unit_luminance_linear_rec709: [1.0; 3],
            perpendicular_illuminance_lux: 100_000.0,
            angular_radius_rad: 0.00465,
            ambient_irradiance_lux: [1000.0; 3],
            background_radiance_cd_m2: [0.0; 3],
        }
    }

    fn golden_bytes(hex: &str) -> [u8; 160] {
        assert_eq!(hex.len(), 320);
        std::array::from_fn(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap())
    }

    #[test]
    fn independent_python_goldens_preserve_every_byte() {
        // Independently generated by Python math/struct/uuid, retained in
        // E1-CONSTANT-LIGHT-F4-PROOF-2026-11-02.tar.gz: oracle.py/oracle.golden.
        // Only payloads are adopted: its hash and build grammars are proof-local.
        let fixtures = [
            (
                [0.0, 1.0, 0.0],
                [1.0; 3],
                "5448524d45314300010000007f00000010000000000040008000000000000001100000000000400080000000000000020000000000000000000000000000f03f0000000000000000000000000000f03f000000000000f03f000000000000f03f00000000006af840ce88d2dee00b733f0000000000408f400000000000408f400000000000408f40000000000000000000000000000000000000000000000000",
            ),
            (
                [-0.6, 0.8000000001, -0.0],
                [0.5, 1.0, 2.47229917],
                "5448524d45314300010000007f0000001000000000004000800000000000000110000000000040008000000000000002599a2c333333e3bf3d8c9e999999e93f00000000000000005cd4f5ffffffdf3f5cd4f5ffffffef3f625682c944c7034000000000006af840ce88d2dee00b733f0000000000408f400000000000408f400000000000408f40000000000000000000000000000000000000000000000000",
            ),
            (
                [0.36, 0.48, -0.8000000001],
                [0.5, 1.0, 2.47229917],
                "5448524d45314300010000007f00000010000000000040008000000000000001100000000000400080000000000000026bec9b703d0ad73f8f907aeb51b8de3f3d8c9e999999e9bf5cd4f5ffffffdf3f5cd4f5ffffffef3f625682c944c7034000000000006af840ce88d2dee00b733f0000000000408f400000000000408f400000000000408f40000000000000000000000000000000000000000000000000",
            ),
        ];
        for (direction, colour, golden) in fixtures {
            let mut input = source();
            input.toward_source_unit = direction;
            input.unit_luminance_linear_rec709 = colour;
            let cooked = ConstantLightProductV1::cook(input).unwrap();
            let expected = golden_bytes(golden);
            assert_eq!(cooked.to_bytes(), expected);
            let decoded = ConstantLightProductV1::from_bytes(&expected).unwrap();
            assert_eq!(decoded, cooked);
            assert_eq!(decoded.to_bytes(), expected);
            assert_eq!(decoded.values().environment_id, ENVIRONMENT);
            assert_eq!(decoded.values().light_id, LIGHT);
        }
    }

    #[test]
    fn every_zero_position_canonicalizes_independently() {
        let mut input = source();
        input.unit_luminance_linear_rec709 = [0.0, 1.0 / 0.7152, 0.0];
        input.perpendicular_illuminance_lux = 0.0;
        input.angular_radius_rad = 0.0;
        input.ambient_irradiance_lux = [0.0; 3];
        let expected = ConstantLightProductV1::cook(input).unwrap().to_bytes();
        // Two direction, two colour and eight other zero channels.
        for signs in 0_u16..4096 {
            let mut signed = input;
            let mut bit = 0;
            for value in numeric_values_mut(&mut signed) {
                if *value == 0.0 {
                    if signs & (1 << bit) != 0 {
                        *value = -0.0;
                    }
                    bit += 1;
                }
            }
            assert_eq!(bit, 12);
            assert_eq!(
                ConstantLightProductV1::cook(signed).unwrap().to_bytes(),
                expected
            );
        }
    }

    #[test]
    fn independent_distinct_channel_golden_fixes_ambient_and_background_order() {
        // Python struct.pack('<14d', 0.,-1.,0.,1.,1.,1.,7.,.125,
        //                    1.,2.,3.,4.,5.,6.) plus the documented header/UUIDs.
        let expected = golden_bytes(
            "5448524d45314300010000007f00000010000000000040008000000000000001100000000000400080000000000000020000000000000000000000000000f0bf0000000000000000000000000000f03f000000000000f03f000000000000f03f0000000000001c40000000000000c03f000000000000f03f00000000000000400000000000000840000000000000104000000000000014400000000000001840",
        );
        let mut input = source();
        input.toward_source_unit = [0.0, -1.0, 0.0];
        input.perpendicular_illuminance_lux = 7.0;
        input.angular_radius_rad = 0.125;
        input.ambient_irradiance_lux = [1.0, 2.0, 3.0];
        input.background_radiance_cd_m2 = [4.0, 5.0, 6.0];
        assert_eq!(
            ConstantLightProductV1::cook(input).unwrap().to_bytes(),
            expected
        );
        assert_eq!(
            ConstantLightProductV1::from_bytes(&expected)
                .unwrap()
                .values(),
            &input
        );
    }

    #[test]
    fn strict_header_and_identity_validation() {
        let base = ConstantLightProductV1::cook(source()).unwrap().to_bytes();
        for length in 0..160 {
            assert_eq!(
                ConstantLightProductV1::from_bytes(&base[..length]),
                Err(ConstantLightError::Length)
            );
        }
        let mut extra = base.to_vec();
        extra.push(0);
        assert_eq!(
            ConstantLightProductV1::from_bytes(&extra),
            Err(ConstantLightError::Length)
        );
        for (offset, value, error) in [
            (0, b'X', ConstantLightError::Magic),
            (7, 1, ConstantLightError::Magic),
            (8, 2, ConstantLightError::Version),
            (9, 1, ConstantLightError::Version),
            (10, 1, ConstantLightError::Flags),
            (11, 1, ConstantLightError::Flags),
            (12, 0x7e, ConstantLightError::UnsupportedMask),
            (12, 0xff, ConstantLightError::UnsupportedMask),
            (15, 1, ConstantLightError::UnsupportedMask),
        ] {
            let mut bytes = base;
            bytes[offset] = value;
            assert_eq!(ConstantLightProductV1::from_bytes(&bytes), Err(error));
        }
        for offset in [16, 32] {
            for version in [0, 9, 15] {
                let mut bytes = base;
                bytes[offset + 6] = version << 4;
                assert_eq!(
                    ConstantLightProductV1::from_bytes(&bytes),
                    Err(ConstantLightError::Uuid)
                );
            }
            for variant in [0, 0x40, 0xc0] {
                let mut bytes = base;
                bytes[offset + 8] = variant;
                assert_eq!(
                    ConstantLightProductV1::from_bytes(&bytes),
                    Err(ConstantLightError::Uuid)
                );
            }
            for version in 1..=8 {
                let mut bytes = base;
                bytes[offset + 6] = version << 4;
                assert!(ConstantLightProductV1::from_bytes(&bytes).is_ok());
            }
        }
        let mut collision = source();
        collision.light_id = collision.environment_id;
        assert_eq!(
            ConstantLightProductV1::cook(collision),
            Err(ConstantLightError::IdentityCollision)
        );
        let mut bytes = base;
        bytes[32..48].copy_from_slice(&ENVIRONMENT);
        assert_eq!(
            ConstantLightProductV1::from_bytes(&bytes),
            Err(ConstantLightError::IdentityCollision)
        );
    }

    #[test]
    fn every_numeric_slot_rejects_nonfinite_negative_zero_and_negative_quantities() {
        let base = ConstantLightProductV1::cook(source()).unwrap().to_bytes();
        for index in 0..14 {
            for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let mut bytes = base;
                bytes[48 + index * 8..56 + index * 8].copy_from_slice(&value.to_le_bytes());
                assert_eq!(
                    ConstantLightProductV1::from_bytes(&bytes),
                    Err(ConstantLightError::Nonfinite)
                );
            }
            let mut bytes = base;
            bytes[48 + index * 8..56 + index * 8].copy_from_slice(&(-0.0_f64).to_le_bytes());
            assert_eq!(
                ConstantLightProductV1::from_bytes(&bytes),
                Err(ConstantLightError::NegativeZero)
            );
            if index >= 3 {
                bytes[48 + index * 8..56 + index * 8].copy_from_slice(&(-1.0_f64).to_le_bytes());
                assert_eq!(
                    ConstantLightProductV1::from_bytes(&bytes),
                    Err(ConstantLightError::NegativeQuantity)
                );
            }
        }
    }

    #[test]
    fn residual_bounds_are_checked_without_repairing_bytes() {
        let base = ConstantLightProductV1::cook(source()).unwrap().to_bytes();
        for (delta, accepted) in [(0.9e-12, true), (1.1e-12, false)] {
            let mut bytes = base;
            bytes[56..64].copy_from_slice(&(1.0_f64 + delta).to_le_bytes());
            assert_eq!(ConstantLightProductV1::from_bytes(&bytes).is_ok(), accepted);
            if accepted {
                assert_eq!(
                    ConstantLightProductV1::from_bytes(&bytes)
                        .unwrap()
                        .to_bytes(),
                    bytes
                );
            }
            let mut bytes = base;
            for index in 0..3 {
                bytes[72 + index * 8..80 + index * 8]
                    .copy_from_slice(&(1.0_f64 + delta).to_le_bytes());
            }
            assert_eq!(ConstantLightProductV1::from_bytes(&bytes).is_ok(), accepted);
        }
    }

    #[test]
    fn full_finite_range_and_signed_direction_survive_roundtrip() {
        let mut input = source();
        input.toward_source_unit = [0.0, -1.0, 0.0];
        input.perpendicular_illuminance_lux = f64::MAX;
        input.angular_radius_rad = f64::from_bits(1);
        input.ambient_irradiance_lux = [f64::MAX, f64::MIN_POSITIVE, f64::from_bits(1)];
        input.background_radiance_cd_m2 = [f64::from_bits(1), f64::MAX, f64::MIN_POSITIVE];
        let cooked = ConstantLightProductV1::cook(input).unwrap();
        assert_eq!(cooked.values(), &input);
        assert_eq!(
            ConstantLightProductV1::from_bytes(&cooked.to_bytes()).unwrap(),
            cooked
        );
    }

    #[test]
    fn invalid_numeric_input_never_becomes_a_plausible_product() {
        for index in 0..14 {
            let mut input = source();
            *numeric_values_mut(&mut input).nth(index).unwrap() = f64::NAN;
            assert_eq!(
                ConstantLightProductV1::cook(input),
                Err(ConstantLightError::Nonfinite)
            );
            if index >= 3 {
                *numeric_values_mut(&mut input).nth(index).unwrap() = -1.0;
                assert_eq!(
                    ConstantLightProductV1::cook(input),
                    Err(ConstantLightError::NegativeQuantity)
                );
            }
        }
        let mut input = source();
        input.toward_source_unit = [0.0; 3];
        assert!(ConstantLightProductV1::cook(input).is_err());
        input = source();
        input.unit_luminance_linear_rec709 = [0.0; 3];
        assert!(ConstantLightProductV1::cook(input).is_err());
        input = source();
        input.toward_source_unit = [f64::MAX; 3];
        assert_eq!(
            ConstantLightProductV1::cook(input),
            Err(ConstantLightError::Normalization)
        );
        input = source();
        input.unit_luminance_linear_rec709 = [-1.0; 3];
        assert_eq!(
            ConstantLightProductV1::cook(input),
            Err(ConstantLightError::NegativeQuantity)
        );
    }
}
