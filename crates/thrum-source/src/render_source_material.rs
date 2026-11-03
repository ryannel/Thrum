//! Strict local material meaning for the inactive shared render-v1 decoder.
//!
//! Borrowed raw fields determine schema-ordered diagnostics without a JSON
//! tree. Texture UUIDs are declarations; namespace closure, slot transfer
//! formats and F4 target support are checked by later owners.

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::render_json::{
    IdentifiedObjectError, SourceText, parse_identified_object, parse_raw_object, uuid_bytes,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AlphaMode {
    Opaque,
    Mask,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SamplerFilter {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SamplerAddress {
    ClampToEdge,
    Repeat,
    MirrorRepeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SamplerFacts {
    pub(crate) min_filter: SamplerFilter,
    pub(crate) mag_filter: SamplerFilter,
    pub(crate) address_u: SamplerAddress,
    pub(crate) address_v: SamplerAddress,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaterialFacts {
    pub(crate) id: [u8; 16],
    pub(crate) alpha_mode: AlphaMode,
    pub(crate) two_sided: bool,
    pub(crate) sampler: SamplerFacts,
    // Opaque has no cutoff; defaults cannot invent an authored value.
    pub(crate) alpha_cutoff: Option<f64>,
    pub(crate) base_factor_rgba: [f64; 4],
    pub(crate) emissive_radiance_factor_cd_m2: [f64; 3],
    pub(crate) metallic_factor: f64,
    pub(crate) roughness_factor: f64,
    pub(crate) occlusion_strength: f64,
    pub(crate) base_texture: Option<[u8; 16]>,
    pub(crate) orm_texture: Option<[u8; 16]>,
    pub(crate) emissive_texture: Option<[u8; 16]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum MaterialField {
    Id,
    AlphaMode,
    TwoSided,
    Sampler,
    AlphaCutoff,
    BaseFactor,
    EmissiveFactor,
    MetallicFactor,
    RoughnessFactor,
    OcclusionStrength,
    BaseTexture,
    OrmTexture,
    EmissiveTexture,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum MaterialProblem {
    Json(serde_json::Error),
    Missing,
    Duplicate,
    Unknown(String),
    InvalidId,
    InvalidAlphaMode,
    InvalidReference,
    Forbidden,
    Nonfinite,
    OutOfRange,
    Sampler(SamplerDecodeError),
}

#[derive(Debug)]
pub(crate) struct MaterialDecodeError {
    pub(crate) id: Option<[u8; 16]>,
    pub(crate) field: MaterialField,
    pub(crate) problem: MaterialProblem,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum SamplerField {
    MinFilter,
    MagFilter,
    AddressU,
    AddressV,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum SamplerProblem {
    Json(serde_json::Error),
    Missing,
    Duplicate,
    Unknown(String),
    InvalidFilter,
    InvalidAddress,
}

#[derive(Debug)]
pub(crate) struct SamplerDecodeError {
    pub(crate) field: SamplerField,
    pub(crate) problem: SamplerProblem,
}

const MATERIAL_FIELDS: [&str; 13] = [
    "id",
    "alpha_mode",
    "two_sided",
    "sampler",
    "alpha_cutoff",
    "base_factor_rgba",
    "emissive_radiance_factor_cd_m2",
    "metallic_factor",
    "roughness_factor",
    "occlusion_strength",
    "base_texture",
    "orm_texture",
    "emissive_texture",
];
const MATERIAL_FIELD_ORDER: [MaterialField; 13] = [
    MaterialField::Id,
    MaterialField::AlphaMode,
    MaterialField::TwoSided,
    MaterialField::Sampler,
    MaterialField::AlphaCutoff,
    MaterialField::BaseFactor,
    MaterialField::EmissiveFactor,
    MaterialField::MetallicFactor,
    MaterialField::RoughnessFactor,
    MaterialField::OcclusionStrength,
    MaterialField::BaseTexture,
    MaterialField::OrmTexture,
    MaterialField::EmissiveTexture,
];
const SAMPLER_FIELDS: [&str; 4] = ["min_filter", "mag_filter", "address_u", "address_v"];
const SAMPLER_FIELD_ORDER: [SamplerField; 4] = [
    SamplerField::MinFilter,
    SamplerField::MagFilter,
    SamplerField::AddressU,
    SamplerField::AddressV,
];

fn material_error(
    id: Option<[u8; 16]>,
    field: MaterialField,
    problem: MaterialProblem,
) -> MaterialDecodeError {
    MaterialDecodeError { id, field, problem }
}

/// Checks source syntax and local numeric meaning without resolving edges or
/// applying target restrictions (including unequal masked sampler filters).
pub(crate) fn decode_material_v1(raw: &RawValue) -> Result<MaterialFacts, MaterialDecodeError> {
    let object = parse_identified_object(raw, &MATERIAL_FIELDS).map_err(|error| match error {
        IdentifiedObjectError::Json(source) => {
            material_error(None, MaterialField::Id, MaterialProblem::Json(source))
        }
        IdentifiedObjectError::MissingId => {
            material_error(None, MaterialField::Id, MaterialProblem::Missing)
        }
        IdentifiedObjectError::InvalidId => {
            material_error(None, MaterialField::Id, MaterialProblem::InvalidId)
        }
        IdentifiedObjectError::Duplicate { index, id } => {
            material_error(id, MATERIAL_FIELD_ORDER[index], MaterialProblem::Duplicate)
        }
    })?;
    let id = object.id;
    let fields = object.fields;
    let error = |field, problem| material_error(Some(id), field, problem);
    let read = |index: usize| {
        fields.values[index]
            .ok_or_else(|| error(MATERIAL_FIELD_ORDER[index], MaterialProblem::Missing))
    };

    let alpha_text: SourceText<'_> = serde_json::from_str(read(1)?.get())
        .map_err(|source| error(MaterialField::AlphaMode, MaterialProblem::Json(source)))?;
    let alpha_mode = match alpha_text.0.as_ref() {
        "opaque" => AlphaMode::Opaque,
        "mask" => AlphaMode::Mask,
        _ => {
            return Err(error(
                MaterialField::AlphaMode,
                MaterialProblem::InvalidAlphaMode,
            ));
        }
    };
    let two_sided: bool = serde_json::from_str(read(2)?.get())
        .map_err(|source| error(MaterialField::TwoSided, MaterialProblem::Json(source)))?;
    let sampler = decode_sampler(read(3)?)
        .map_err(|cause| error(MaterialField::Sampler, MaterialProblem::Sampler(cause)))?;
    let alpha_cutoff = match alpha_mode {
        AlphaMode::Opaque if fields.values[4].is_some() => {
            return Err(error(
                MaterialField::AlphaCutoff,
                MaterialProblem::Forbidden,
            ));
        }
        AlphaMode::Opaque => None,
        AlphaMode::Mask => Some(
            unit_scalar(read(4)?).map_err(|problem| error(MaterialField::AlphaCutoff, problem))?,
        ),
    };
    let base_factor_rgba = fields.values[5]
        .map(|raw| finite_array::<4>(raw, Some(1.0)))
        .transpose()
        .map_err(|problem| error(MaterialField::BaseFactor, problem))?
        .unwrap_or([1.0; 4]);
    let emissive_radiance_factor_cd_m2 = fields.values[6]
        .map(|raw| finite_array::<3>(raw, None))
        .transpose()
        .map_err(|problem| error(MaterialField::EmissiveFactor, problem))?
        .unwrap_or([0.0; 3]);
    let scalar = |index: usize, default| {
        fields.values[index]
            .map(unit_scalar)
            .transpose()
            .map_err(|problem| error(MATERIAL_FIELD_ORDER[index], problem))
            .map(|value| value.unwrap_or(default))
    };
    let metallic_factor = scalar(7, 0.0)?;
    let roughness_factor = scalar(8, 1.0)?;
    let occlusion_strength = scalar(9, 1.0)?;
    let reference = |index: usize| {
        fields.values[index]
            .map(|raw| {
                let text: SourceText<'_> =
                    serde_json::from_str(raw.get()).map_err(MaterialProblem::Json)?;
                uuid_bytes(&text.0).ok_or(MaterialProblem::InvalidReference)
            })
            .transpose()
            .map_err(|problem| error(MATERIAL_FIELD_ORDER[index], problem))
    };
    let base_texture = reference(10)?;
    let orm_texture = reference(11)?;
    let emissive_texture = reference(12)?;
    if let Some(unknown) = fields.unknown {
        return Err(error(
            MaterialField::Unknown,
            MaterialProblem::Unknown(unknown.0.into_owned()),
        ));
    }
    Ok(MaterialFacts {
        id,
        alpha_mode,
        two_sided,
        sampler,
        alpha_cutoff,
        base_factor_rgba,
        emissive_radiance_factor_cd_m2,
        metallic_factor,
        roughness_factor,
        occlusion_strength,
        base_texture,
        orm_texture,
        emissive_texture,
    })
}

fn unit_scalar(raw: &RawValue) -> Result<f64, MaterialProblem> {
    let value: f64 = serde_json::from_str(raw.get()).map_err(MaterialProblem::Json)?;
    if !value.is_finite() {
        return Err(MaterialProblem::Nonfinite);
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(MaterialProblem::OutOfRange);
    }
    Ok(value)
}

fn finite_array<const N: usize>(
    raw: &RawValue,
    upper: Option<f64>,
) -> Result<[f64; N], MaterialProblem>
where
    for<'de> [f64; N]: Deserialize<'de>,
{
    let values: [f64; N] = serde_json::from_str(raw.get()).map_err(MaterialProblem::Json)?;
    if values.iter().any(|value| !value.is_finite()) {
        return Err(MaterialProblem::Nonfinite);
    }
    if values
        .iter()
        .any(|&value| value < 0.0 || upper.is_some_and(|upper| value > upper))
    {
        return Err(MaterialProblem::OutOfRange);
    }
    Ok(values)
}

fn decode_sampler(raw: &RawValue) -> Result<SamplerFacts, SamplerDecodeError> {
    let error = |field, problem| SamplerDecodeError { field, problem };
    let fields = parse_raw_object(raw.get(), &SAMPLER_FIELDS)
        .map_err(|source| error(SamplerField::MinFilter, SamplerProblem::Json(source)))?;
    if let Some(index) = fields.duplicate.iter().position(|&duplicate| duplicate) {
        return Err(error(SAMPLER_FIELD_ORDER[index], SamplerProblem::Duplicate));
    }
    let read_text = |index: usize| {
        let raw = fields.values[index]
            .ok_or_else(|| error(SAMPLER_FIELD_ORDER[index], SamplerProblem::Missing))?;
        serde_json::from_str::<SourceText<'_>>(raw.get())
            .map_err(|source| error(SAMPLER_FIELD_ORDER[index], SamplerProblem::Json(source)))
    };
    let filter = |index| match read_text(index)?.0.as_ref() {
        "nearest" => Ok(SamplerFilter::Nearest),
        "linear" => Ok(SamplerFilter::Linear),
        _ => Err(error(
            SAMPLER_FIELD_ORDER[index],
            SamplerProblem::InvalidFilter,
        )),
    };
    let address = |index| match read_text(index)?.0.as_ref() {
        "clamp-to-edge" => Ok(SamplerAddress::ClampToEdge),
        "repeat" => Ok(SamplerAddress::Repeat),
        "mirror-repeat" => Ok(SamplerAddress::MirrorRepeat),
        _ => Err(error(
            SAMPLER_FIELD_ORDER[index],
            SamplerProblem::InvalidAddress,
        )),
    };
    let min_filter = filter(0)?;
    let mag_filter = filter(1)?;
    let address_u = address(2)?;
    let address_v = address(3)?;
    if let Some(unknown) = fields.unknown {
        return Err(error(
            SamplerField::Unknown,
            SamplerProblem::Unknown(unknown.0.into_owned()),
        ));
    }
    Ok(SamplerFacts {
        min_filter,
        mag_filter,
        address_u,
        address_v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "a6b9c300-0000-4000-8000-000000000016";
    const TEXTURE: &str = "a6b9c300-0000-4000-8000-000000000017";
    const SAMPLER: &str = r#"{"min_filter":"nearest","mag_filter":"linear","address_u":"repeat","address_v":"clamp-to-edge"}"#;

    fn raw(text: &str) -> &RawValue {
        serde_json::from_str(text).unwrap()
    }

    fn material(mode: &str, extra: &str) -> String {
        format!(
            r#"{{"id":"{ID}","alpha_mode":"{mode}","two_sided":false,"sampler":{SAMPLER}{extra}}}"#
        )
    }

    fn decode(text: &str) -> Result<MaterialFacts, MaterialDecodeError> {
        decode_material_v1(raw(text))
    }

    #[test]
    fn native_defaults_and_complete_authored_facts() {
        let defaults = decode(&material("opaque", "")).unwrap();
        assert_eq!(defaults.id[15], 0x16);
        assert_eq!(defaults.alpha_mode, AlphaMode::Opaque);
        assert!(!defaults.two_sided);
        assert_eq!(defaults.alpha_cutoff, None);
        assert_eq!(defaults.base_factor_rgba, [1.0; 4]);
        assert_eq!(defaults.emissive_radiance_factor_cd_m2, [0.0; 3]);
        assert_eq!(defaults.metallic_factor, 0.0);
        assert_eq!(defaults.roughness_factor, 1.0);
        assert_eq!(defaults.occlusion_strength, 1.0);
        assert_eq!(defaults.base_texture, None);
        assert_eq!(defaults.orm_texture, None);
        assert_eq!(defaults.emissive_texture, None);

        let authored = material(
            "mask",
            &format!(
                r#", "alpha_cutoff":0.5,"base_factor_rgba":[0,0.25,0.5,1],"emissive_radiance_factor_cd_m2":[5e-324,12.5,1.7976931348623157e308],"metallic_factor":0.75,"roughness_factor":0,"occlusion_strength":0.125,"base_texture":"{TEXTURE}","orm_texture":"{TEXTURE}","emissive_texture":"{TEXTURE}""#
            ),
        )
        .replace("\"two_sided\":false", "\"two_sided\":true");
        let facts = decode(&authored).unwrap();
        assert_eq!(facts.alpha_mode, AlphaMode::Mask);
        assert!(facts.two_sided);
        assert_eq!(facts.alpha_cutoff, Some(0.5));
        assert_eq!(facts.base_factor_rgba, [0.0, 0.25, 0.5, 1.0]);
        assert_eq!(
            facts.emissive_radiance_factor_cd_m2,
            [f64::from_bits(1), 12.5, f64::MAX]
        );
        assert_eq!(facts.metallic_factor, 0.75);
        assert_eq!(facts.roughness_factor, 0.0);
        assert_eq!(facts.occlusion_strength, 0.125);
        let texture = uuid_bytes(TEXTURE).unwrap();
        assert_eq!(facts.base_texture, Some(texture));
        assert_eq!(facts.orm_texture, Some(texture));
        assert_eq!(facts.emissive_texture, Some(texture));
    }

    #[test]
    fn cutoff_presence_is_alpha_mode_dependent() {
        let missing = decode(&material("mask", "")).unwrap_err();
        assert_eq!(missing.field, MaterialField::AlphaCutoff);
        assert!(matches!(missing.problem, MaterialProblem::Missing));
        for value in ["0", "1", "null", "\"bad\""] {
            let error =
                decode(&material("opaque", &format!(",\"alpha_cutoff\":{value}"))).unwrap_err();
            assert_eq!(error.field, MaterialField::AlphaCutoff);
            assert!(matches!(error.problem, MaterialProblem::Forbidden));
        }
        for cutoff in ["0", "1"] {
            assert!(decode(&material("mask", &format!(",\"alpha_cutoff\":{cutoff}"))).is_ok());
        }
        let error = decode(&material("blend", "")).unwrap_err();
        assert_eq!(error.field, MaterialField::AlphaMode);
        assert!(matches!(error.problem, MaterialProblem::InvalidAlphaMode));
    }

    #[test]
    fn sampler_accepts_every_source_combination_including_unequal_masked_filters() {
        for min in ["nearest", "linear"] {
            for mag in ["nearest", "linear"] {
                for u in ["clamp-to-edge", "repeat", "mirror-repeat"] {
                    for v in ["clamp-to-edge", "repeat", "mirror-repeat"] {
                        let sampler = format!(
                            r#"{{"address_v":"{v}","mag_filter":"{mag}","address_u":"{u}","min_filter":"{min}"}}"#
                        );
                        let text =
                            material("mask", ",\"alpha_cutoff\":0.5").replace(SAMPLER, &sampler);
                        let facts = decode(&text).unwrap();
                        assert_eq!(
                            facts.sampler.min_filter == SamplerFilter::Nearest,
                            min == "nearest"
                        );
                        assert_eq!(
                            facts.sampler.mag_filter == SamplerFilter::Nearest,
                            mag == "nearest"
                        );
                        let expected = |value| match value {
                            "clamp-to-edge" => SamplerAddress::ClampToEdge,
                            "repeat" => SamplerAddress::Repeat,
                            "mirror-repeat" => SamplerAddress::MirrorRepeat,
                            _ => unreachable!(),
                        };
                        assert_eq!(facts.sampler.address_u, expected(u));
                        assert_eq!(facts.sampler.address_v, expected(v));
                    }
                }
            }
        }
    }

    #[test]
    fn optional_null_wrong_shapes_and_numeric_domains_refuse() {
        for (name, field) in [
            ("base_factor_rgba", MaterialField::BaseFactor),
            (
                "emissive_radiance_factor_cd_m2",
                MaterialField::EmissiveFactor,
            ),
            ("metallic_factor", MaterialField::MetallicFactor),
            ("roughness_factor", MaterialField::RoughnessFactor),
            ("occlusion_strength", MaterialField::OcclusionStrength),
            ("base_texture", MaterialField::BaseTexture),
            ("orm_texture", MaterialField::OrmTexture),
            ("emissive_texture", MaterialField::EmissiveTexture),
        ] {
            let error = decode(&material("opaque", &format!(",\"{name}\":null"))).unwrap_err();
            assert_eq!(error.field, field);
            assert!(matches!(error.problem, MaterialProblem::Json(_)));
        }
        for (extra, field) in [
            (r#", "base_factor_rgba":[1,1,1]"#, MaterialField::BaseFactor),
            (
                r#", "base_factor_rgba":[1,1,1,1,1]"#,
                MaterialField::BaseFactor,
            ),
            (
                r#", "base_factor_rgba":[1,1,1,1.0000000000000002]"#,
                MaterialField::BaseFactor,
            ),
            (
                r#", "base_factor_rgba":[1,1,-5e-324,1]"#,
                MaterialField::BaseFactor,
            ),
            (
                r#", "emissive_radiance_factor_cd_m2":[0,0,-5e-324]"#,
                MaterialField::EmissiveFactor,
            ),
            (
                r#", "emissive_radiance_factor_cd_m2":[0,0,1e400]"#,
                MaterialField::EmissiveFactor,
            ),
            (
                r#", "emissive_radiance_factor_cd_m2":[0,0]"#,
                MaterialField::EmissiveFactor,
            ),
            (
                r#", "metallic_factor":-5e-324"#,
                MaterialField::MetallicFactor,
            ),
            (
                r#", "roughness_factor":1.0000000000000002"#,
                MaterialField::RoughnessFactor,
            ),
            (
                r#", "occlusion_strength":"1""#,
                MaterialField::OcclusionStrength,
            ),
            (r#", "base_texture":"invalid""#, MaterialField::BaseTexture),
            (
                r#", "orm_texture":"A6B9C300-0000-4000-8000-000000000017""#,
                MaterialField::OrmTexture,
            ),
        ] {
            assert_eq!(
                decode(&material("opaque", extra)).unwrap_err().field,
                field,
                "{extra}"
            );
        }
        for extra in [
            r#", "alpha_cutoff":null"#,
            r#", "alpha_cutoff":-5e-324"#,
            r#", "alpha_cutoff":1.0000000000000002"#,
        ] {
            assert_eq!(
                decode(&material("mask", extra)).unwrap_err().field,
                MaterialField::AlphaCutoff
            );
        }
        let facts = decode(&material("mask", r#", "alpha_cutoff":-0.0,"base_factor_rgba":[-0.0,5e-324,0,1],"roughness_factor":5e-324"#)).unwrap();
        assert_eq!(facts.alpha_cutoff.unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(facts.base_factor_rgba[0].to_bits(), (-0.0_f64).to_bits());
        assert_eq!(facts.base_factor_rgba[1].to_bits(), 1);
        assert_eq!(facts.roughness_factor.to_bits(), 1);
    }

    #[test]
    fn escaped_and_reordered_source_has_identical_facts() {
        let text = material(
            "mask",
            &format!(r#", "alpha_cutoff":0.25,"base_texture":"{TEXTURE}""#),
        );
        let escaped = text
            .replace("alpha_mode", r"alpha\u005fmode")
            .replace("mask", r"m\u0061sk")
            .replace("two_sided", r"two\u005fsided")
            .replace("min_filter", r"min\u005ffilter")
            .replace("nearest", r"n\u0065arest")
            .replace("clamp-to-edge", r"clamp-to-\u0065dge")
            .replace("a6b9c300", r"\u00616b9c300");
        let reordered = format!(
            r#"{{"base_texture":"{TEXTURE}","alpha_cutoff":0.25,"sampler":{{"address_v":"clamp-to-edge","address_u":"repeat","mag_filter":"linear","min_filter":"nearest"}},"two_sided":false,"alpha_mode":"mask","id":"{ID}"}}"#
        );
        assert_eq!(decode(&text).unwrap(), decode(&escaped).unwrap());
        assert_eq!(decode(&text).unwrap(), decode(&reordered).unwrap());
    }

    #[test]
    fn duplicate_priority_unknown_final_and_schema_order_ignore_encounter_order() {
        let duplicate = material(
            "invalid",
            r#", "roughness_factor":0,"roughness\u005ffactor":1,"two_sided":true"#,
        );
        let error = decode(&duplicate).unwrap_err();
        assert_eq!(error.id, uuid_bytes(ID));
        assert_eq!(error.field, MaterialField::TwoSided);
        assert!(matches!(error.problem, MaterialProblem::Duplicate));
        let duplicate_id = material("opaque", &format!(",\"i\\u0064\":\"{ID}\""));
        let error = decode(&duplicate_id).unwrap_err();
        assert_eq!(error.id, None);
        assert_eq!(error.field, MaterialField::Id);
        assert!(matches!(error.problem, MaterialProblem::Duplicate));

        for extra in [
            r#", "roughness_factor":-1,"metallic_factor":2,"a_unknown":0"#,
            r#", "a_unknown":0,"metallic_factor":2,"roughness_factor":-1"#,
        ] {
            assert_eq!(
                decode(&material("opaque", extra)).unwrap_err().field,
                MaterialField::MetallicFactor
            );
        }
        let error = decode(&material(
            "opaque",
            r#", "z_unknown":0,"a\u005funknown":0,"a_unknown":1"#,
        ))
        .unwrap_err();
        assert_eq!(error.field, MaterialField::Unknown);
        assert!(matches!(error.problem, MaterialProblem::Unknown(name) if name == "a_unknown"));
        for extra in [
            r#", "normal_texture":null"#,
            r#", "normal_scale":1"#,
            r#", "smooth_normal":true"#,
        ] {
            assert_eq!(
                decode(&material("opaque", extra)).unwrap_err().field,
                MaterialField::Unknown
            );
        }
    }

    #[test]
    fn nested_sampler_is_strict_and_its_duplicate_priority_is_local() {
        for (sampler, field, is_duplicate) in [
            (
                r#"{"min_filter":"bad","mag_filter":"linear","address_u":"repeat","address_v":"repeat","address\u005fv":"repeat"}"#,
                SamplerField::AddressV,
                true,
            ),
            (
                r#"{"address_v":"bad","min_filter":"bad","mag_filter":"bad","address_u":"bad"}"#,
                SamplerField::MinFilter,
                false,
            ),
            (
                r#"{"min_filter":"nearest","address_u":"repeat","address_v":"repeat"}"#,
                SamplerField::MagFilter,
                false,
            ),
            (
                r#"{"min_filter":"nearest","mag_filter":"linear","address_u":"bad","address_v":"repeat"}"#,
                SamplerField::AddressU,
                false,
            ),
            (
                r#"{"min_filter":"nearest","mag_filter":"linear","address_u":"repeat","address_v":"repeat","mip_filter":"nearest"}"#,
                SamplerField::Unknown,
                false,
            ),
            (
                r#"["nearest","linear","repeat","repeat"]"#,
                SamplerField::MinFilter,
                false,
            ),
            ("null", SamplerField::MinFilter, false),
        ] {
            let text = material("mask", ",\"alpha_cutoff\":-1").replace(SAMPLER, sampler);
            let error = decode(&text).unwrap_err();
            assert_eq!(error.field, MaterialField::Sampler);
            let MaterialProblem::Sampler(cause) = error.problem else {
                panic!("expected nested sampler error")
            };
            assert_eq!(cause.field, field);
            assert_eq!(
                matches!(cause.problem, SamplerProblem::Duplicate),
                is_duplicate
            );
        }
        let duplicate_sampler = r#"{"min_filter":"nearest","min_filter":"linear"}"#;
        let text = material("invalid", "").replace(SAMPLER, duplicate_sampler);
        assert_eq!(decode(&text).unwrap_err().field, MaterialField::AlphaMode);
        let text = material("opaque", "")
            .replace(SAMPLER, duplicate_sampler)
            .replace("\"two_sided\":false", "\"two_sided\":null");
        assert_eq!(decode(&text).unwrap_err().field, MaterialField::TwoSided);
    }

    #[test]
    fn required_fields_and_object_only_grammar_refuse() {
        let plain = material("opaque", "");
        for (name, field) in [
            ("id", MaterialField::Id),
            ("alpha_mode", MaterialField::AlphaMode),
            ("two_sided", MaterialField::TwoSided),
            ("sampler", MaterialField::Sampler),
        ] {
            let token = match name {
                "id" => format!(r#""id":"{ID}","#),
                "alpha_mode" => r#""alpha_mode":"opaque","#.to_owned(),
                "two_sided" => r#""two_sided":false,"#.to_owned(),
                "sampler" => format!(r#", "sampler":{SAMPLER}"#).replace(", ", ","),
                _ => unreachable!(),
            };
            let error = decode(&plain.replace(&token, "")).unwrap_err();
            assert_eq!(error.field, field, "{name}");
            assert!(matches!(error.problem, MaterialProblem::Missing));
        }
        for text in ["null", "[]", r#"{"id":null}"#, r#"{"id":"invalid"}"#] {
            let error = decode(text).unwrap_err();
            assert_eq!(error.id, None);
            assert_eq!(error.field, MaterialField::Id);
        }
    }
}
