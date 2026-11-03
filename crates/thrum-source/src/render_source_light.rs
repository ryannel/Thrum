//! Strict local point-light meaning for the inactive shared render-v1 decoder.
//!
//! Fixed-count borrowed raw fields provide schema-ordered diagnostics without
//! a heap tree. Authored values remain unchanged; W1 pose ownership, recipe
//! counts and F4 representation support belong to later stages.

use serde_json::value::RawValue;

use crate::render_json::{IdentifiedObjectError, parse_identified_object};
use crate::render_light_luminance_exact::has_unit_luminance;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LocalLightFacts {
    pub(crate) id: [u8; 16],
    pub(crate) position_m: [f64; 3],
    pub(crate) luminous_intensity_cd: f64,
    pub(crate) unit_luminance_linear_rec709: [f64; 3],
    pub(crate) radius_m: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum LocalLightField {
    Id,
    Position,
    LuminousIntensity,
    UnitLuminance,
    Radius,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum LocalLightProblem {
    Json(serde_json::Error),
    Missing,
    Duplicate,
    Unknown(String),
    InvalidId,
    Nonfinite,
    OutOfRange,
    InvalidUnitLuminance,
}

#[derive(Debug)]
pub(crate) struct LocalLightDecodeError {
    pub(crate) id: Option<[u8; 16]>,
    pub(crate) field: LocalLightField,
    pub(crate) problem: LocalLightProblem,
}

const LIGHT_FIELDS: [&str; 5] = [
    "id",
    "position_m",
    "luminous_intensity_cd",
    "unit_luminance_linear_rec709",
    "radius_m",
];
const LIGHT_FIELD_ORDER: [LocalLightField; 5] = [
    LocalLightField::Id,
    LocalLightField::Position,
    LocalLightField::LuminousIntensity,
    LocalLightField::UnitLuminance,
    LocalLightField::Radius,
];

fn light_error(
    id: Option<[u8; 16]>,
    field: LocalLightField,
    problem: LocalLightProblem,
) -> LocalLightDecodeError {
    LocalLightDecodeError { id, field, problem }
}

/// Checks source meaning, including exact rational unit luminance over the
/// decoded f64 components, independently of recipe counts and target support.
pub(crate) fn decode_local_light_v1(
    raw: &RawValue,
) -> Result<LocalLightFacts, LocalLightDecodeError> {
    let object = parse_identified_object(raw, &LIGHT_FIELDS).map_err(|error| match error {
        IdentifiedObjectError::Json(source) => {
            light_error(None, LocalLightField::Id, LocalLightProblem::Json(source))
        }
        IdentifiedObjectError::MissingId => {
            light_error(None, LocalLightField::Id, LocalLightProblem::Missing)
        }
        IdentifiedObjectError::InvalidId => {
            light_error(None, LocalLightField::Id, LocalLightProblem::InvalidId)
        }
        IdentifiedObjectError::Duplicate { index, id } => {
            light_error(id, LIGHT_FIELD_ORDER[index], LocalLightProblem::Duplicate)
        }
    })?;
    let id = object.id;
    let fields = object.fields;
    let error = |field, problem| light_error(Some(id), field, problem);
    let read = |index: usize| {
        fields.values[index]
            .ok_or_else(|| error(LIGHT_FIELD_ORDER[index], LocalLightProblem::Missing))
    };

    let position_m: [f64; 3] = serde_json::from_str(read(1)?.get())
        .map_err(|source| error(LocalLightField::Position, LocalLightProblem::Json(source)))?;
    if position_m.iter().any(|value| !value.is_finite()) {
        return Err(error(
            LocalLightField::Position,
            LocalLightProblem::Nonfinite,
        ));
    }
    if position_m.iter().any(|value| value.abs() > 1_000_000.0) {
        return Err(error(
            LocalLightField::Position,
            LocalLightProblem::OutOfRange,
        ));
    }
    let luminous_intensity_cd = nonnegative_scalar(read(2)?)
        .map_err(|problem| error(LocalLightField::LuminousIntensity, problem))?;
    let unit_luminance_linear_rec709: [f64; 3] =
        serde_json::from_str(read(3)?.get()).map_err(|source| {
            error(
                LocalLightField::UnitLuminance,
                LocalLightProblem::Json(source),
            )
        })?;
    if unit_luminance_linear_rec709
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(error(
            LocalLightField::UnitLuminance,
            LocalLightProblem::Nonfinite,
        ));
    }
    if unit_luminance_linear_rec709
        .iter()
        .any(|value| *value < 0.0)
    {
        return Err(error(
            LocalLightField::UnitLuminance,
            LocalLightProblem::OutOfRange,
        ));
    }
    if !has_unit_luminance(unit_luminance_linear_rec709) {
        return Err(error(
            LocalLightField::UnitLuminance,
            LocalLightProblem::InvalidUnitLuminance,
        ));
    }
    let radius_m =
        nonnegative_scalar(read(4)?).map_err(|problem| error(LocalLightField::Radius, problem))?;
    if let Some(unknown) = fields.unknown {
        return Err(error(
            LocalLightField::Unknown,
            LocalLightProblem::Unknown(unknown.0.into_owned()),
        ));
    }
    Ok(LocalLightFacts {
        id,
        position_m,
        luminous_intensity_cd,
        unit_luminance_linear_rec709,
        radius_m,
    })
}

fn nonnegative_scalar(raw: &RawValue) -> Result<f64, LocalLightProblem> {
    let value: f64 = serde_json::from_str(raw.get()).map_err(LocalLightProblem::Json)?;
    if !value.is_finite() {
        return Err(LocalLightProblem::Nonfinite);
    }
    if value < 0.0 {
        return Err(LocalLightProblem::OutOfRange);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_json::uuid_bytes;

    const ID: &str = "a6b9c300-0000-4000-8000-000000000018";
    const LIGHT: &str = r#"{"id":"a6b9c300-0000-4000-8000-000000000018","position_m":[1,-2,3],"luminous_intensity_cd":1200,"unit_luminance_linear_rec709":[1,1,1],"radius_m":0.25}"#;

    fn decode(text: &str) -> Result<LocalLightFacts, LocalLightDecodeError> {
        let raw: &RawValue = serde_json::from_str(text).unwrap();
        decode_local_light_v1(raw)
    }

    #[test]
    fn authored_facts_preserve_extrema_signed_zero_and_subnormals() {
        let facts = decode(LIGHT).unwrap();
        assert_eq!(facts.id, uuid_bytes(ID).unwrap());
        assert_eq!(facts.position_m, [1.0, -2.0, 3.0]);
        assert_eq!(facts.luminous_intensity_cd, 1200.0);
        assert_eq!(facts.unit_luminance_linear_rec709, [1.0; 3]);
        assert_eq!(facts.radius_m, 0.25);

        let extremes = LIGHT
            .replace("[1,-2,3]", "[-1000000,1000000,-0.0]")
            .replace("1200", "1.7976931348623157e308")
            .replace("[1,1,1]", "[-0.0,1,3.9445983379501386]")
            .replace("0.25", "5e-324");
        let facts = decode(&extremes).unwrap();
        assert_eq!(facts.position_m, [-1_000_000.0, 1_000_000.0, -0.0]);
        assert_eq!(facts.position_m[2].to_bits(), (-0.0_f64).to_bits());
        assert_eq!(facts.luminous_intensity_cd, f64::MAX);
        assert_eq!(
            facts.unit_luminance_linear_rec709[0].to_bits(),
            (-0.0_f64).to_bits()
        );
        assert_eq!(facts.radius_m.to_bits(), 1);

        let tiny = LIGHT
            .replace("[1,-2,3]", "[5e-324,-5e-324,0]")
            .replace("1200", "-0.0")
            .replace("0.25", "1.7976931348623157e308");
        let facts = decode(&tiny).unwrap();
        assert_eq!(facts.position_m[0].to_bits(), 1);
        assert_eq!(
            facts.position_m[1].to_bits(),
            (-f64::from_bits(1)).to_bits()
        );
        assert_eq!(facts.luminous_intensity_cd.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(facts.radius_m, f64::MAX);

        let tiny_colour = LIGHT
            .replace("[1,1,1]", "[5e-324,1,3.9445983379501386]")
            .replace("1200", "5e-324")
            .replace("0.25", "-0.0");
        let facts = decode(&tiny_colour).unwrap();
        assert_eq!(facts.unit_luminance_linear_rec709[0].to_bits(), 1);
        assert_eq!(facts.luminous_intensity_cd.to_bits(), 1);
        assert_eq!(facts.radius_m.to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn exact_luminance_disagreements_are_decided_from_decoded_source() {
        // Independent Python Fraction.from_float oracle, using integer weights
        // /5000 and tolerance 1/10^9. Rounded binary64 arithmetic disagrees.
        for (colour, rgb, expected) in [
            (
                "[4.703668857008466,0,0]",
                [4.703668857008466, 0.0, 0.0],
                false,
            ),
            (
                "[0,0,13.850415526315789]",
                [0.0, 0.0, 13.850415526315789],
                true,
            ),
        ] {
            let rounded = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
            assert_ne!((rounded - 1.0_f64).abs() <= 1e-9, expected);
            match decode(&LIGHT.replace("[1,1,1]", colour)) {
                Ok(facts) => {
                    assert!(expected);
                    assert_eq!(facts.unit_luminance_linear_rec709, rgb);
                }
                Err(error) => {
                    assert!(!expected);
                    assert_eq!(error.field, LocalLightField::UnitLuminance);
                    assert!(matches!(
                        error.problem,
                        LocalLightProblem::InvalidUnitLuminance
                    ));
                }
            }
        }
    }

    #[test]
    fn exact_luminance_endpoints_and_neighbours() {
        // Archived independent rational endpoint oracle from the predicate
        // proof; serializing and decoding must preserve those exact f64 bits.
        for (bits, expected) in [
            (0x4012d08e91a042a0, false),
            (0x4012d08e91a042a1, true),
            (0x4012d08e9241e088, true),
            (0x4012d08e9241e089, false),
        ] {
            let colour = format!("[{},0,0]", f64::from_bits(bits));
            let decoded = decode(&LIGHT.replace("[1,1,1]", &colour));
            assert_eq!(decoded.is_ok(), expected, "{bits:x}");
            if let Ok(facts) = decoded {
                assert_eq!(facts.unit_luminance_linear_rec709[0].to_bits(), bits);
            }
        }
    }

    #[test]
    fn numeric_domains_and_wrong_types_refuse_at_their_fields() {
        for (original, replacement, field) in [
            (
                "[1,-2,3]",
                "[1000000.0000000001,0,0]",
                LocalLightField::Position,
            ),
            (
                "[1,-2,3]",
                "[-1000000.0000000001,0,0]",
                LocalLightField::Position,
            ),
            ("[1,-2,3]", "[1e400,0,0]", LocalLightField::Position),
            ("[1,-2,3]", "null", LocalLightField::Position),
            ("[1,-2,3]", "[1,2]", LocalLightField::Position),
            ("1200", "-5e-324", LocalLightField::LuminousIntensity),
            ("1200", "1e400", LocalLightField::LuminousIntensity),
            ("1200", "\"1200\"", LocalLightField::LuminousIntensity),
            ("1200", "null", LocalLightField::LuminousIntensity),
            ("[1,1,1]", "[-5e-324,1,1]", LocalLightField::UnitLuminance),
            ("[1,1,1]", "[0,0,0]", LocalLightField::UnitLuminance),
            ("[1,1,1]", "[1e400,1,1]", LocalLightField::UnitLuminance),
            ("[1,1,1]", "[1,1,1,1]", LocalLightField::UnitLuminance),
            ("[1,1,1]", "null", LocalLightField::UnitLuminance),
            ("0.25", "-5e-324", LocalLightField::Radius),
            ("0.25", "1e400", LocalLightField::Radius),
            ("0.25", "false", LocalLightField::Radius),
            ("0.25", "null", LocalLightField::Radius),
        ] {
            let error = decode(&LIGHT.replace(original, replacement)).unwrap_err();
            assert_eq!(error.id, uuid_bytes(ID));
            assert_eq!(error.field, field, "{replacement}");
        }
    }

    #[test]
    fn escaped_equivalence_and_reordering_preserve_facts() {
        let escaped = LIGHT
            .replace("position_m", r"position\u005fm")
            .replace("luminous_intensity_cd", r"luminous\u005fintensity_cd")
            .replace(
                "unit_luminance_linear_rec709",
                r"unit_luminance_linear_rec\u003709",
            )
            .replace("radius_m", r"radius\u005fm")
            .replace("a6b9c300", r"\u00616b9c300");
        let reordered = format!(
            r#"{{"radius_m":0.25,"unit_luminance_linear_rec709":[1,1,1],"luminous_intensity_cd":1200,"position_m":[1,-2,3],"id":"{ID}"}}"#
        );
        assert_eq!(decode(LIGHT).unwrap(), decode(&escaped).unwrap());
        assert_eq!(decode(LIGHT).unwrap(), decode(&reordered).unwrap());
    }

    #[test]
    fn recognized_duplicates_precede_semantics_and_use_schema_order() {
        let repeated = LIGHT.replace("[1,-2,3]", "null").replace(
            "\"radius_m\":0.25",
            r#""radius_m":0.25,"radius\u005fm":0,"unit_luminance_linear_rec709":[0,0,0]"#,
        );
        let error = decode(&repeated).unwrap_err();
        assert_eq!(error.id, uuid_bytes(ID));
        assert_eq!(error.field, LocalLightField::UnitLuminance);
        assert!(matches!(error.problem, LocalLightProblem::Duplicate));
        let repeated_id = LIGHT.replace(
            "\"radius_m\":0.25",
            &format!(r#""radius_m":0.25,"i\u0064":"{ID}""#),
        );
        let error = decode(&repeated_id).unwrap_err();
        assert_eq!(error.id, None);
        assert_eq!(error.field, LocalLightField::Id);
        assert!(matches!(error.problem, LocalLightProblem::Duplicate));
    }

    #[test]
    fn semantic_priority_and_unknown_refusal_are_canonical() {
        for text in [
            format!(
                r#"{{"id":"{ID}","radius_m":-1,"unit_luminance_linear_rec709":[0,0,0],"luminous_intensity_cd":-1,"position_m":[1000001,0,0],"a_unknown":null}}"#
            ),
            format!(
                r#"{{"a_unknown":null,"position_m":[1000001,0,0],"luminous_intensity_cd":-1,"unit_luminance_linear_rec709":[0,0,0],"radius_m":-1,"id":"{ID}"}}"#
            ),
        ] {
            let error = decode(&text).unwrap_err();
            assert_eq!(error.field, LocalLightField::Position);
            assert!(matches!(error.problem, LocalLightProblem::OutOfRange));
        }
        let unknown = LIGHT.replace(
            "\"radius_m\":0.25",
            r#""radius_m":0.25,"z_unknown":false,"a\u005funknown":null,"a_unknown":true"#,
        );
        let error = decode(&unknown).unwrap_err();
        assert_eq!(error.field, LocalLightField::Unknown);
        assert!(matches!(error.problem, LocalLightProblem::Unknown(name) if name == "a_unknown"));
        let bad_radius = unknown.replace("0.25", "-1");
        assert_eq!(
            decode(&bad_radius).unwrap_err().field,
            LocalLightField::Radius
        );
    }

    #[test]
    fn every_field_is_required_and_only_objects_are_accepted() {
        for (token, field) in [
            (format!(r#""id":"{ID}","#), LocalLightField::Id),
            (
                r#""position_m":[1,-2,3],"#.to_owned(),
                LocalLightField::Position,
            ),
            (
                r#""luminous_intensity_cd":1200,"#.to_owned(),
                LocalLightField::LuminousIntensity,
            ),
            (
                r#""unit_luminance_linear_rec709":[1,1,1],"#.to_owned(),
                LocalLightField::UnitLuminance,
            ),
            (
                r#", "radius_m":0.25"#.replace(", ", ","),
                LocalLightField::Radius,
            ),
        ] {
            let error = decode(&LIGHT.replace(&token, "")).unwrap_err();
            assert_eq!(error.field, field);
            assert!(matches!(error.problem, LocalLightProblem::Missing));
        }
        for text in ["null", "[]", r#"{"id":null}"#, r#"{"id":"invalid"}"#] {
            let error = decode(text).unwrap_err();
            assert_eq!(error.id, None);
            assert_eq!(error.field, LocalLightField::Id);
        }
    }
}
