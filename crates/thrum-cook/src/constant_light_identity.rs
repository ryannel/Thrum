//! Candidate E1 byte identities only. This private test module does not access
//! source stores, retain pins, publish content or establish cache/build authority.
//! Class and lock bytes deliberately have test-only names: their trusted
//! descriptors and the lease-bearing publication API remain unselected.

use crate::{ConstantLightError, ConstantLightInputV1, ConstantLightProductV1};
use sha2::{Digest, Sha256};

const COOK_DOMAIN: &[u8] = b"thrum.e1.constant-light.cook-key.v1\0";
const PRODUCT_DOMAIN: &[u8] = b"thrum.e1.constant-light.product-revision.v1\0";
const BINDING_DOMAIN: &[u8] = b"thrum.e1.constant-light.binding-revision.v1\0";
const RECIPE_LEN: usize = COOK_DOMAIN.len() + 255;

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(super) struct $name(pub(super) [u8; 32]);
    };
}
digest_type!(CookKey);
digest_type!(BlobHash);
digest_type!(ProductRevision);
digest_type!(BindingRevision);
digest_type!(TestCookClass);
digest_type!(TestProfileLock);

/// Hashes supplied by the F3 descriptor lookup used for admission, never copied
/// built-in constants. This codec neither performs nor authenticates admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DefinitionHashes {
    pub environment: [u8; 32],
    pub light: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum IdentityError {
    Length,
    Magic,
    Version,
    Format,
    Flags,
    Uuid,
    IdentityCollision,
    Product(ConstantLightError),
    ProductRevision,
    BindingRevision,
    CookKey,
    BlobHash,
    ExpectedProduct,
    Context,
}

/// Encode authored known values before normalization. Validation reuses the
/// selected payload safety checks; F3 source-domain/tolerance admission is still
/// the caller's responsibility. All work is fixed count with stack arrays.
pub(super) fn canonical_recipe(
    input: &ConstantLightInputV1,
    definitions: DefinitionHashes,
    class: TestCookClass,
) -> Result<[u8; RECIPE_LEN], IdentityError> {
    ConstantLightProductV1::cook(*input).map_err(IdentityError::Product)?;
    Ok(encode_checked_recipe(input, definitions, class))
}

// Only called after the same input passed the selected product safety checks.
fn encode_checked_recipe(
    input: &ConstantLightInputV1,
    definitions: DefinitionHashes,
    class: TestCookClass,
) -> [u8; RECIPE_LEN] {
    let mut bytes = [0; RECIPE_LEN];
    let mut cursor = 0;
    let mut append = |value: &[u8]| {
        bytes[cursor..cursor + value.len()].copy_from_slice(value);
        cursor += value.len();
    };
    append(COOK_DOMAIN);
    for _ in 0..5 {
        append(&1_u16.to_le_bytes());
    }
    append(&[1]); // incoming-environment-sun
    append(&0x7f_u32.to_le_bytes());
    append(&input.environment_id);
    append(&input.light_id);
    append(&definitions.environment);
    append(&definitions.light);
    append(&class.0);
    for value in numeric_values(input) {
        let bits = if value == 0.0 { 0 } else { value.to_bits() };
        append(&bits.to_le_bytes());
    }
    debug_assert_eq!(cursor, RECIPE_LEN);
    bytes
}

fn numeric_values(input: &ConstantLightInputV1) -> impl Iterator<Item = f64> + '_ {
    input
        .toward_source_unit
        .iter()
        .chain(&input.unit_luminance_linear_rec709)
        .chain(std::iter::once(&input.perpendicular_illuminance_lux))
        .chain(std::iter::once(&input.angular_radius_rad))
        .chain(&input.ambient_irradiance_lux)
        .chain(&input.background_radiance_cd_m2)
        .copied()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ProductManifest {
    pub cook_key: CookKey,
    pub blob_hash: BlobHash,
}

impl ProductManifest {
    pub const BYTE_LEN: usize = 80;

    pub fn to_bytes(self) -> [u8; Self::BYTE_LEN] {
        let mut bytes = [0; Self::BYTE_LEN];
        bytes[..8].copy_from_slice(b"THRE1PM\0");
        bytes[8..10].copy_from_slice(&1_u16.to_le_bytes());
        bytes[10..12].copy_from_slice(&1_u16.to_le_bytes());
        bytes[12..16].copy_from_slice(&160_u32.to_le_bytes());
        bytes[16..48].copy_from_slice(&self.cook_key.0);
        bytes[48..80].copy_from_slice(&self.blob_hash.0);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IdentityError> {
        if bytes.len() != Self::BYTE_LEN {
            return Err(IdentityError::Length);
        }
        if &bytes[..8] != b"THRE1PM\0" {
            return Err(IdentityError::Magic);
        }
        if bytes[8..10] != 1_u16.to_le_bytes() {
            return Err(IdentityError::Version);
        }
        if bytes[10..12] != 1_u16.to_le_bytes() || bytes[12..16] != 160_u32.to_le_bytes() {
            return Err(IdentityError::Format);
        }
        Ok(Self {
            cook_key: CookKey(array(bytes, 16)),
            blob_hash: BlobHash(array(bytes, 48)),
        })
    }

    pub fn revision(self) -> ProductRevision {
        ProductRevision(domain_hash(PRODUCT_DOMAIN, &self.to_bytes()))
    }
}

/// Detached test expectation, with no accepted-pin or publication authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ExpectedProduct {
    pub payload: [u8; ConstantLightProductV1::BYTE_LEN],
    pub manifest: ProductManifest,
}

impl ExpectedProduct {
    pub fn cook(
        input: &ConstantLightInputV1,
        definitions: DefinitionHashes,
        class: TestCookClass,
    ) -> Result<Self, IdentityError> {
        let payload = ConstantLightProductV1::cook(*input)
            .map_err(IdentityError::Product)?
            .to_bytes();
        let recipe = encode_checked_recipe(input, definitions, class);
        Ok(Self {
            manifest: ProductManifest {
                cook_key: CookKey(Sha256::digest(recipe).into()),
                blob_hash: BlobHash(Sha256::digest(payload).into()),
            },
            payload,
        })
    }

    /// Check byte names, strict payload grammar and the recooked expectation.
    /// A self-consistent replacement manifest/blob still refuses here.
    pub fn verify_association(
        &self,
        manifest_bytes: &[u8],
        named_revision: ProductRevision,
        payload: &[u8],
    ) -> Result<(), IdentityError> {
        let manifest = ProductManifest::from_bytes(manifest_bytes)?;
        if manifest.revision() != named_revision {
            return Err(IdentityError::ProductRevision);
        }
        if manifest.cook_key != self.manifest.cook_key {
            return Err(IdentityError::CookKey);
        }
        if payload.len() != ConstantLightProductV1::BYTE_LEN {
            return Err(IdentityError::Length);
        }
        if BlobHash(Sha256::digest(payload).into()) != manifest.blob_hash {
            return Err(IdentityError::BlobHash);
        }
        ConstantLightProductV1::from_bytes(payload).map_err(IdentityError::Product)?;
        if payload != self.payload || manifest != self.manifest {
            return Err(IdentityError::ExpectedProduct);
        }
        Ok(())
    }
}

/// Raw expected binding bytes; source digest presence is not an accepted root,
/// store-owner check, source lease or profile attestation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct BindingContext {
    pub source_revision: [u8; 32],
    pub project_id: [u8; 16],
    pub scene_id: [u8; 16],
    pub environment_id: [u8; 16],
    pub light_id: [u8; 16],
    pub scene_definition_hash: [u8; 32],
    pub profile_lock: TestProfileLock,
}

impl BindingContext {
    fn validate(self) -> Result<(), IdentityError> {
        let ids = [
            self.project_id,
            self.scene_id,
            self.environment_id,
            self.light_id,
        ];
        for (index, id) in ids.iter().enumerate() {
            if !(1..=8).contains(&(id[6] >> 4)) || id[8] & 0xc0 != 0x80 {
                return Err(IdentityError::Uuid);
            }
            if ids[..index].contains(id) {
                return Err(IdentityError::IdentityCollision);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Binding {
    pub context: BindingContext,
    pub product_revision: ProductRevision,
}

impl Binding {
    pub const BYTE_LEN: usize = 204;

    pub fn new(
        context: BindingContext,
        product_revision: ProductRevision,
    ) -> Result<Self, IdentityError> {
        context.validate()?;
        Ok(Self {
            context,
            product_revision,
        })
    }

    pub fn to_bytes(self) -> [u8; Self::BYTE_LEN] {
        let mut bytes = [0; Self::BYTE_LEN];
        bytes[..8].copy_from_slice(b"THRE1BD\0");
        bytes[8..10].copy_from_slice(&1_u16.to_le_bytes());
        bytes[12..44].copy_from_slice(&self.context.source_revision);
        bytes[44..60].copy_from_slice(&self.context.project_id);
        bytes[60..76].copy_from_slice(&self.context.scene_id);
        bytes[76..92].copy_from_slice(&self.context.environment_id);
        bytes[92..108].copy_from_slice(&self.context.light_id);
        bytes[108..140].copy_from_slice(&self.context.scene_definition_hash);
        bytes[140..172].copy_from_slice(&self.context.profile_lock.0);
        bytes[172..204].copy_from_slice(&self.product_revision.0);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IdentityError> {
        if bytes.len() != Self::BYTE_LEN {
            return Err(IdentityError::Length);
        }
        if &bytes[..8] != b"THRE1BD\0" {
            return Err(IdentityError::Magic);
        }
        if bytes[8..10] != 1_u16.to_le_bytes() {
            return Err(IdentityError::Version);
        }
        if bytes[10..12] != [0, 0] {
            return Err(IdentityError::Flags);
        }
        Self::new(
            BindingContext {
                source_revision: array(bytes, 12),
                project_id: array(bytes, 44),
                scene_id: array(bytes, 60),
                environment_id: array(bytes, 76),
                light_id: array(bytes, 92),
                scene_definition_hash: array(bytes, 108),
                profile_lock: TestProfileLock(array(bytes, 140)),
            },
            ProductRevision(array(bytes, 172)),
        )
    }

    pub fn revision(self) -> BindingRevision {
        BindingRevision(domain_hash(BINDING_DOMAIN, &self.to_bytes()))
    }

    pub fn verify(
        bytes: &[u8],
        named_revision: BindingRevision,
        context: BindingContext,
        expected_product: ProductRevision,
    ) -> Result<Self, IdentityError> {
        context.validate()?;
        let binding = Self::from_bytes(bytes)?;
        if binding.revision() != named_revision {
            return Err(IdentityError::BindingRevision);
        }
        if binding.context != context {
            return Err(IdentityError::Context);
        }
        if binding.product_revision != expected_product {
            return Err(IdentityError::ProductRevision);
        }
        Ok(binding)
    }
}

fn array<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    bytes[offset..offset + N]
        .try_into()
        .expect("checked fixed encoding length")
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(bytes);
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(first: u8) -> [u8; 16] {
        [
            first, 0x5e, 0xf6, 0x44, 0x72, 0x25, 0x47, 0x70, 0x9f, 0x4d, 0xe1, 0xd9, 0xc0, 0xa7,
            0xd5, 0x15,
        ]
    }

    fn input() -> ConstantLightInputV1 {
        ConstantLightInputV1 {
            environment_id: id(0x4a),
            light_id: id(0x5a),
            toward_source_unit: [0.0, 1.0, 0.0],
            unit_luminance_linear_rec709: [1.0; 3],
            perpendicular_illuminance_lux: 120_000.0,
            angular_radius_rad: 0.00465,
            ambient_irradiance_lux: [4.0, 5.0, 6.0],
            background_radiance_cd_m2: [7.0, 8.0, 9.0],
        }
    }

    fn definitions() -> DefinitionHashes {
        DefinitionHashes {
            environment: std::array::from_fn(|index| index as u8),
            light: std::array::from_fn(|index| index as u8 + 32),
        }
    }

    fn expected() -> ExpectedProduct {
        ExpectedProduct::cook(&input(), definitions(), TestCookClass([0xc1; 32])).unwrap()
    }

    fn context() -> BindingContext {
        BindingContext {
            source_revision: [0x51; 32],
            project_id: id(0x2a),
            scene_id: id(0x3a),
            environment_id: id(0x4a),
            light_id: id(0x5a),
            scene_definition_hash: [0xd1; 32],
            profile_lock: TestProfileLock([0xe1; 32]),
        }
    }

    fn unhex<const N: usize>(text: &str) -> [u8; N] {
        assert_eq!(text.len(), N * 2);
        std::array::from_fn(|index| {
            u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).unwrap()
        })
    }

    // Independent Python 3 struct/hashlib/uuid oracle, using only this fixture:
    // ids = [uuid.UUID(f'{n}a5ef644-7225-4770-9f4d-e1d9c0a7d515').bytes
    //        for n in (2,3,4,5)]
    // values = [0.,1.,0.,1.,1.,1.,120000.,.00465,4.,5.,6.,7.,8.,9.]
    // recipe = b'thrum.e1.constant-light.cook-key.v1\0' +
    //   struct.pack('<5HBI',1,1,1,1,1,1,127) + ids[2]+ids[3] +
    //   bytes(range(32))+bytes(range(32,64))+bytes([193])*32 +
    //   struct.pack('<14d', *values)
    // payload = b'THRME1C\0'+struct.pack('<HHI',1,0,127)+ids[2]+ids[3]+
    //   struct.pack('<14d', *values)
    // manifest = b'THRE1PM\0'+struct.pack('<HHI',1,1,160)+
    //   sha256(recipe).digest()+sha256(payload).digest()
    // product = sha256(b'thrum.e1.constant-light.product-revision.v1\0'+manifest).digest()
    // binding = b'THRE1BD\0'+struct.pack('<HH',1,0)+bytes([81])*32+
    //   b''.join(ids)+bytes([209])*32+bytes([225])*32+product
    // binding_revision = sha256(b'thrum.e1.constant-light.binding-revision.v1\0'+binding)
    #[test]
    fn independent_recipe_manifest_binding_and_digest_goldens() {
        let recipe = canonical_recipe(&input(), definitions(), TestCookClass([0xc1; 32])).unwrap();
        assert_eq!(
            recipe,
            unhex::<291>(concat!(
                "746872756d2e65312e636f6e7374616e742d6c696768742e636f6f6b2d6b65792e763100",
                "01000100010001000100017f000000",
                "4a5ef644722547709f4de1d9c0a7d5155a5ef644722547709f4de1d9c0a7d515",
                "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
                "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f",
                "c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1",
                "0000000000000000000000000000f03f0000000000000000",
                "000000000000f03f000000000000f03f000000000000f03f",
                "00000000004cfd40ce88d2dee00b733f",
                "000000000000104000000000000014400000000000001840",
                "0000000000001c4000000000000020400000000000002240"
            ))
        );
        let expected = expected();
        assert_eq!(
            expected.manifest.cook_key.0,
            unhex("326a92b9fab9efca26568afcc40d626338ae31728275adeca01a256d4b13344e")
        );
        assert_eq!(
            expected.manifest.blob_hash.0,
            unhex("77524a5f3cc2da1048e98dced020fb4d8a3b445981eb195a02a4e3a90bf42d64")
        );
        let manifest_bytes = unhex::<80>(concat!(
            "5448524531504d0001000100a0000000",
            "326a92b9fab9efca26568afcc40d626338ae31728275adeca01a256d4b13344e",
            "77524a5f3cc2da1048e98dced020fb4d8a3b445981eb195a02a4e3a90bf42d64"
        ));
        assert_eq!(expected.manifest.to_bytes(), manifest_bytes);
        assert_eq!(
            ProductManifest::from_bytes(&manifest_bytes),
            Ok(expected.manifest)
        );
        assert_eq!(
            expected.manifest.revision().0,
            unhex("4eb1db429d078b4802e2fa790e14dcfcb132c7ff4a600a8992ad8b12e45b32fa")
        );
        let binding = Binding::new(context(), expected.manifest.revision()).unwrap();
        let binding_bytes = unhex::<204>(concat!(
            "544852453142440001000000",
            "5151515151515151515151515151515151515151515151515151515151515151",
            "2a5ef644722547709f4de1d9c0a7d5153a5ef644722547709f4de1d9c0a7d515",
            "4a5ef644722547709f4de1d9c0a7d5155a5ef644722547709f4de1d9c0a7d515",
            "d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1",
            "e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1",
            "4eb1db429d078b4802e2fa790e14dcfcb132c7ff4a600a8992ad8b12e45b32fa"
        ));
        assert_eq!(binding.to_bytes(), binding_bytes);
        assert_eq!(Binding::from_bytes(&binding_bytes), Ok(binding));
        assert_eq!(
            binding.revision().0,
            unhex("24946153c12c27865808f8e0b7cad36db0f32e7c060b06dba592406d4dfcfcd0")
        );
        assert_eq!(
            Binding::verify(
                &binding_bytes,
                binding.revision(),
                context(),
                expected.manifest.revision()
            ),
            Ok(binding)
        );
        assert_eq!(
            expected.verify_association(
                &manifest_bytes,
                expected.manifest.revision(),
                &expected.payload
            ),
            Ok(())
        );
    }

    #[test]
    fn distinct_direction_and_colour_components_have_independent_golden() {
        let mut raw = input();
        raw.toward_source_unit = [2.0 / 7.0, 3.0 / 7.0, 6.0 / 7.0];
        raw.unit_luminance_linear_rec709 = [0.5, 1.0, (1.0 - ((0.2126 * 0.5) + 0.7152)) / 0.0722];
        // Independent Python oracle uses the prior fixture's prefix and:
        // v = [2./7.,3./7.,6./7.,.5,1.,(1.-(.2126*.5+.7152))/.0722,
        //      120000.,.00465,4.,5.,6.,7.,8.,9.]
        // suffix = struct.pack('<14d', *v)
        // key = hashlib.sha256(prefix + suffix).digest()
        // Distinct components make any within-triplet permutation observable.
        let suffix = unhex::<112>(concat!(
            "922449922449d23fdbb66ddbb66ddb3fdbb66ddbb66deb3f",
            "000000000000e03f000000000000f03f426865c944c70340",
            "00000000004cfd40ce88d2dee00b733f",
            "000000000000104000000000000014400000000000001840",
            "0000000000001c4000000000000020400000000000002240"
        ));
        let recipe = canonical_recipe(&raw, definitions(), TestCookClass([0xc1; 32])).unwrap();
        assert_eq!(&recipe[RECIPE_LEN - suffix.len()..], &suffix);
        let cooked = ExpectedProduct::cook(&raw, definitions(), TestCookClass([0xc1; 32])).unwrap();
        assert_eq!(
            cooked.manifest.cook_key.0,
            unhex("545cbf39400f4ff69466524ff9228104b752cb6737f31b95aa07aa598d71b793")
        );
    }

    #[test]
    fn key_preserves_authored_bits_and_only_canonicalizes_zero() {
        let base = expected();
        let mut signed = input();
        signed.toward_source_unit[0] = -0.0;
        signed.toward_source_unit[2] = -0.0;
        assert_eq!(
            ExpectedProduct::cook(&signed, definitions(), TestCookClass([0xc1; 32])).unwrap(),
            base
        );
        let mut raw = input();
        raw.toward_source_unit[1] = f64::from_bits(1.0_f64.to_bits() + 1);
        let changed =
            ExpectedProduct::cook(&raw, definitions(), TestCookClass([0xc1; 32])).unwrap();
        assert_eq!(changed.payload, base.payload); // Normalization erases the authored difference.
        assert_ne!(changed.manifest.cook_key, base.manifest.cook_key);
        let mut tiny = input();
        tiny.ambient_irradiance_lux[0] = f64::from_bits(1);
        let recipe = canonical_recipe(&tiny, definitions(), TestCookClass([0xc1; 32])).unwrap();
        let numeric_start = RECIPE_LEN - 112;
        assert_eq!(
            &recipe[numeric_start + 64..numeric_start + 72],
            &1_u64.to_le_bytes()
        );
    }

    #[test]
    fn all_selected_semantic_inputs_invalidate_key() {
        let base = expected().manifest.cook_key;
        for index in 0..14 {
            let mut raw = input();
            let values: [&mut f64; 14] = {
                let [x, y, z] = &mut raw.toward_source_unit;
                let [r, g, b] = &mut raw.unit_luminance_linear_rec709;
                let [a0, a1, a2] = &mut raw.ambient_irradiance_lux;
                let [b0, b1, b2] = &mut raw.background_radiance_cd_m2;
                [
                    x,
                    y,
                    z,
                    r,
                    g,
                    b,
                    &mut raw.perpendicular_illuminance_lux,
                    &mut raw.angular_radius_rad,
                    a0,
                    a1,
                    a2,
                    b0,
                    b1,
                    b2,
                ]
            };
            *values.into_iter().nth(index).unwrap() += 0.000001;
            assert_ne!(
                ExpectedProduct::cook(&raw, definitions(), TestCookClass([0xc1; 32]))
                    .unwrap()
                    .manifest
                    .cook_key,
                base
            );
        }
        for environment in [true, false] {
            let mut raw = input();
            if environment {
                raw.environment_id[0] ^= 1;
            } else {
                raw.light_id[0] ^= 1;
            }
            assert_ne!(
                ExpectedProduct::cook(&raw, definitions(), TestCookClass([0xc1; 32]))
                    .unwrap()
                    .manifest
                    .cook_key,
                base
            );
            let mut definitions = definitions();
            if environment {
                definitions.environment[0] ^= 1;
            } else {
                definitions.light[0] ^= 1;
            }
            assert_ne!(
                ExpectedProduct::cook(&input(), definitions, TestCookClass([0xc1; 32]))
                    .unwrap()
                    .manifest
                    .cook_key,
                base
            );
        }
        assert_ne!(
            ExpectedProduct::cook(&input(), definitions(), TestCookClass([0xc2; 32]))
                .unwrap()
                .manifest
                .cook_key,
            base
        );
    }

    #[test]
    fn malformed_manifest_and_binding_refuse_without_repair() {
        let manifest = expected().manifest.to_bytes();
        for length in 0..80 {
            assert_eq!(
                ProductManifest::from_bytes(&manifest[..length]),
                Err(IdentityError::Length)
            );
        }
        let mut extra = manifest.to_vec();
        extra.push(0);
        assert_eq!(
            ProductManifest::from_bytes(&extra),
            Err(IdentityError::Length)
        );
        for (offset, error) in [
            (0, IdentityError::Magic),
            (8, IdentityError::Version),
            (10, IdentityError::Format),
            (12, IdentityError::Format),
        ] {
            let mut bytes = manifest;
            bytes[offset] ^= 1;
            assert_eq!(ProductManifest::from_bytes(&bytes), Err(error));
        }
        let binding = Binding::new(context(), expected().manifest.revision())
            .unwrap()
            .to_bytes();
        for length in 0..204 {
            assert_eq!(
                Binding::from_bytes(&binding[..length]),
                Err(IdentityError::Length)
            );
        }
        let mut extra = binding.to_vec();
        extra.push(0);
        assert_eq!(Binding::from_bytes(&extra), Err(IdentityError::Length));
        for (offset, error) in [
            (0, IdentityError::Magic),
            (8, IdentityError::Version),
            (10, IdentityError::Flags),
            (11, IdentityError::Flags),
            (50, IdentityError::Uuid),
            (52, IdentityError::Uuid),
        ] {
            let mut bytes = binding;
            bytes[offset] = if offset == 50 || offset == 52 {
                0
            } else {
                bytes[offset] ^ 1
            };
            assert_eq!(Binding::from_bytes(&bytes), Err(error));
        }
        let mut collision = binding;
        collision[60..76].copy_from_slice(&binding[44..60]);
        assert_eq!(
            Binding::from_bytes(&collision),
            Err(IdentityError::IdentityCollision)
        );
    }

    #[test]
    fn source_and_context_changes_only_rebind_the_same_product() {
        let product = expected();
        let base = Binding::new(context(), product.manifest.revision()).unwrap();
        // Every expected field is checked, including the exact source, scene
        // descriptor and lock. Decode alone cannot know the intended context.
        for offset in [12, 44, 60, 76, 92, 108, 140] {
            let mut bytes = base.to_bytes();
            bytes[offset] ^= 1;
            let changed = Binding::from_bytes(&bytes).unwrap();
            assert_ne!(changed.revision(), base.revision());
            assert_eq!(changed.product_revision, base.product_revision);
            assert_eq!(
                Binding::verify(
                    &bytes,
                    changed.revision(),
                    context(),
                    product.manifest.revision()
                ),
                Err(IdentityError::Context)
            );
        }
        let mut reordered = base.to_bytes();
        reordered[44..60].copy_from_slice(&base.context.scene_id);
        reordered[60..76].copy_from_slice(&base.context.project_id);
        let decoded = Binding::from_bytes(&reordered).unwrap();
        assert_eq!(
            Binding::verify(
                &reordered,
                decoded.revision(),
                context(),
                base.product_revision
            ),
            Err(IdentityError::Context)
        );
    }

    #[test]
    fn corrupt_cross_wired_and_self_consistent_wrong_products_refuse() {
        let good = expected();
        let manifest = good.manifest.to_bytes();
        let revision = good.manifest.revision();
        assert_eq!(
            good.verify_association(&manifest, ProductRevision([0; 32]), &good.payload),
            Err(IdentityError::ProductRevision)
        );
        let mut corrupt = good.payload;
        corrupt[159] ^= 1;
        assert_eq!(
            good.verify_association(&manifest, revision, &corrupt),
            Err(IdentityError::BlobHash)
        );
        assert_eq!(
            good.verify_association(&manifest, revision, &good.payload[..159]),
            Err(IdentityError::Length)
        );
        let mut different = input();
        different.perpendicular_illuminance_lux = 2.0;
        let other =
            ExpectedProduct::cook(&different, definitions(), TestCookClass([0xc1; 32])).unwrap();
        assert_eq!(
            good.verify_association(
                &other.manifest.to_bytes(),
                other.manifest.revision(),
                &other.payload
            ),
            Err(IdentityError::CookKey)
        );
        // Adversarial replacement has the requested key and its own valid blob.
        let replacement = ProductManifest {
            cook_key: good.manifest.cook_key,
            blob_hash: other.manifest.blob_hash,
        };
        assert_eq!(
            good.verify_association(
                &replacement.to_bytes(),
                replacement.revision(),
                &other.payload
            ),
            Err(IdentityError::ExpectedProduct)
        );
        let mut malformed = good.payload;
        malformed[10] = 1;
        let malformed_manifest = ProductManifest {
            cook_key: good.manifest.cook_key,
            blob_hash: BlobHash(Sha256::digest(malformed).into()),
        };
        assert_eq!(
            good.verify_association(
                &malformed_manifest.to_bytes(),
                malformed_manifest.revision(),
                &malformed
            ),
            Err(IdentityError::Product(ConstantLightError::Flags))
        );
        let binding = Binding::new(context(), revision).unwrap();
        assert_eq!(
            Binding::verify(
                &binding.to_bytes(),
                BindingRevision([0; 32]),
                context(),
                revision
            ),
            Err(IdentityError::BindingRevision)
        );
        assert_eq!(
            Binding::verify(
                &binding.to_bytes(),
                binding.revision(),
                context(),
                other.manifest.revision()
            ),
            Err(IdentityError::ProductRevision)
        );
        let mut swapped = manifest;
        swapped[16..48].copy_from_slice(&good.manifest.blob_hash.0);
        swapped[48..80].copy_from_slice(&good.manifest.cook_key.0);
        let decoded = ProductManifest::from_bytes(&swapped).unwrap();
        assert_eq!(
            good.verify_association(&swapped, decoded.revision(), &good.payload),
            Err(IdentityError::CookKey)
        );
    }

    #[test]
    fn invalid_raw_numeric_and_uuid_domains_refuse() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut raw = input();
            raw.angular_radius_rad = value;
            assert_eq!(
                canonical_recipe(&raw, definitions(), TestCookClass([0; 32])),
                Err(IdentityError::Product(ConstantLightError::Nonfinite))
            );
        }
        let mut raw = input();
        raw.perpendicular_illuminance_lux = -1.0;
        assert_eq!(
            canonical_recipe(&raw, definitions(), TestCookClass([0; 32])),
            Err(IdentityError::Product(ConstantLightError::NegativeQuantity))
        );
        raw = input();
        raw.environment_id[6] = 0;
        assert_eq!(
            canonical_recipe(&raw, definitions(), TestCookClass([0; 32])),
            Err(IdentityError::Product(ConstantLightError::Uuid))
        );
        raw = input();
        raw.light_id = raw.environment_id;
        assert_eq!(
            canonical_recipe(&raw, definitions(), TestCookClass([0; 32])),
            Err(IdentityError::Product(
                ConstantLightError::IdentityCollision
            ))
        );
    }
}
