//! Direct bounded parsers for the first constant-light engine-owned records.
//!
//! Staged validation uses these private parsers. The immutable staged bytes
//! retain opaque source values exactly; those values have no runtime meaning.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::value::RawValue;

use crate::record::canonical_uuid;
use crate::{MAX_RECORD_BYTES, MAX_RECORD_NODES};

const MAX_STRING_BYTES: usize = 4096;
const MAX_EXTENSIONS: usize = 16;
const MAX_EXTENSION_BYTES: usize = 16 * 1024;
const MAX_TOTAL_EXTENSION_BYTES: usize = 256 * 1024;
const MAX_EXTENSION_DEPTH: usize = 16;
const MAX_EXTENSION_NODES: usize = 4096;
const UNIT_TOLERANCE: f64 = 1e-9;

/// Exact immutable descriptor bytes, hashed under the built-in definition domain.
/// The text is an SDK-owned descriptor language, version 1, not editable source.
pub(crate) const SCENE_DEFINITION: &str = concat!(
    "thrum-builtin-definition-v1\ncore.scene@2\n",
    "envelope=id:canonical-rfc4122-uuid,type:core.scene,schema:2,fields:object,documentation?:utf8<=4096\n",
    "scene.label?:utf8<=4096\nenvironment.source?:required-ref<thrum.environment.authored>\n",
    "unknown=refuse;duplicate=refuse;json=strict-finite-i-json;record-bytes<=16777216\n",
);
pub(crate) const ENVIRONMENT_DEFINITION: &str = concat!(
    "thrum-builtin-definition-v1\nthrum.environment.authored@1\n",
    "envelope=id:canonical-rfc4122-uuid,type:thrum.environment.authored,schema:1,fields:object,documentation?:utf8<=4096\n",
    "label?:utf8<=4096\nprofile=constant-light-v1\n",
    "sun.light_ref:required-ref<thrum.light.identity@1>\n",
    "sun.toward_source_unit:finite-f64[3];abs(euclidean-length-1)<=1e-9;length>0;raw-toward-source-world-direction\n",
    "sun.perpendicular_illuminance_lux:finite-nonnegative-f64;raw-unattenuated-perpendicular-illuminance-lux\n",
    "sun.unit_luminance_linear_rec709:finite-nonnegative-f64[3];abs(dot([0.2126,0.7152,0.0722],colour)-1)<=1e-9\n",
    "sun.angular_radius_rad:finite-nonnegative-f64;geometric-radians\n",
    "sun.direction_model=geometric-straight\nsun.applied_attenuation=[]\n",
    "ambient_irradiance_lux:finite-nonnegative-f64[3];isotropic-hemispherical-incident-RGB-irradiance-lux;Lambert-BRDF-divides-by-pi\n",
    "background_radiance_cd_m2:finite-nonnegative-f64[3];independent-linear-Rec709-RGB-radiance-cd-per-m2\n",
    "unsupported:exact-unique-set[wind,cloud_tau,precipitation,air,thermal,moon,air_quality]\n",
    "support=constant-all-time-and-position;interpolation=none;weather-state=none;normalization=cook-not-parse\n",
    "opaque=fields-only;x.thrum.environment.;keys<=16;ascii-key-utf8<=4096;raw-bytes-each<=16384;raw-bytes-total<=262144;aggregate-value-nodes<=4096;container-depth<=16;decoded-string-or-key-utf8<=4096;untouched-raw-bytes-preserved\n",
    "unknown=refuse-except-opaque;duplicate=refuse;json=strict-finite-i-json;record-bytes<=16777216\n",
);
pub(crate) const LIGHT_DEFINITION: &str = concat!(
    "thrum-builtin-definition-v1\nthrum.light.identity@1\n",
    "envelope=id:canonical-rfc4122-uuid,type:thrum.light.identity,schema:1,fields:object,documentation?:utf8<=4096\n",
    "label?:utf8<=4096\ndriver_binding=incoming-environment-sun;ownership=identity-only;transform-or-intensity=refuse\n",
    "unknown=refuse;duplicate=refuse;json=strict-finite-i-json;record-bytes<=16777216\n",
);

pub(crate) struct RecordHeader {
    #[cfg(test)]
    pub(crate) id: String,
    pub(crate) record_type: String,
    pub(crate) schema: u64,
}

/// Read the checked top-level identity and discriminators before typed parsing.
/// Skip fields with a bounded no-tree walk, so dispatch enforces whole-record
/// limits without applying game-owned opaque limits to engine-owned sources.
/// Duplicate nested keys and field meanings remain the selected parser's job.
pub(crate) fn record_header(bytes: &[u8]) -> Result<RecordHeader, serde_json::Error> {
    struct HeaderVisitor<'a> {
        nodes: &'a Cell<usize>,
    }
    impl<'de> Visitor<'de> for HeaderVisitor<'_> {
        type Value = RecordHeader;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a source envelope with unique type and schema discriminators")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let (mut record_type, mut schema) = (None, None);
            let (mut id, mut fields_seen, mut documentation_seen) = (None, false, false);
            while let Some(key) = map.next_key::<Text>()? {
                match key.0.as_str() {
                    "type" if record_type.is_none() => {
                        record_type = Some(
                            map.next_value_seed(CountHeaderSeed::<Text> {
                                nodes: self.nodes,
                                marker: PhantomData,
                            })?
                            .0,
                        )
                    }
                    "schema" if schema.is_none() => {
                        schema = Some(map.next_value_seed(CountHeaderSeed::<u64> {
                            nodes: self.nodes,
                            marker: PhantomData,
                        })?)
                    }
                    "id" if id.is_none() => {
                        id = Some(
                            map.next_value_seed(CountHeaderSeed::<Uuid> {
                                nodes: self.nodes,
                                marker: PhantomData,
                            })?
                            .0,
                        );
                    }
                    "fields" if !fields_seen => {
                        map.next_value_seed(HeaderSkipSeed {
                            depth: 1,
                            nodes: self.nodes,
                        })?;
                        fields_seen = true;
                    }
                    "documentation" if !documentation_seen => {
                        // Legacy game/scene documentation retains its existing
                        // whole-record byte bound. Built-ins additionally check
                        // their 4096-byte text limit in the selected parser.
                        let _: DiscardedHeaderText = map.next_value_seed(CountHeaderSeed {
                            nodes: self.nodes,
                            marker: PhantomData,
                        })?;
                        documentation_seen = true;
                    }
                    _ => return Err(A::Error::custom("duplicate or unknown envelope key")),
                }
            }
            if id.is_none() || !fields_seen {
                return Err(A::Error::custom("missing source envelope field"));
            }
            let id = required(id)?;
            #[cfg(not(test))]
            drop(id);
            Ok(RecordHeader {
                #[cfg(test)]
                id,
                record_type: required(record_type)?,
                schema: required(schema)?,
            })
        }
    }
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(serde_json::Error::custom("record byte limit"));
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let nodes = Cell::new(1); // The root envelope is one value/container.
    let header = serde::Deserializer::deserialize_map(&mut d, HeaderVisitor { nodes: &nodes })?;
    d.end()?;
    Ok(header)
}

// Header traversal retains no container members, keys or scalar strings. Depth
// is the number of enclosing containers: the root envelope is depth 0 and its
// fields value starts at depth 1. Containers at depths 0..63 use the permitted
// 64 levels. Nodes include the envelope and exclude object keys.
const MAX_HEADER_CONTAINER_DEPTH: usize = 64;

fn count_header_node<E: serde::de::Error>(nodes: &Cell<usize>) -> Result<(), E> {
    if nodes.get() >= MAX_RECORD_NODES {
        return Err(E::custom("source header JSON value count limit"));
    }
    nodes.set(nodes.get() + 1);
    Ok(())
}

struct CountHeaderSeed<'a, T> {
    nodes: &'a Cell<usize>,
    marker: PhantomData<T>,
}
impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for CountHeaderSeed<'_, T> {
    type Value = T;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<T, D::Error> {
        count_header_node(self.nodes)?;
        T::deserialize(d)
    }
}

struct DiscardedHeaderText;
impl<'de> Deserialize<'de> for DiscardedHeaderText {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct TextVisitor;
        impl Visitor<'_> for TextVisitor {
            type Value = DiscardedHeaderText;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a source string bounded by the whole-record byte limit")
            }
            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
                if text.len() > MAX_RECORD_BYTES {
                    return Err(E::custom("source header string byte limit"));
                }
                Ok(DiscardedHeaderText)
            }
        }
        d.deserialize_str(TextVisitor)
    }
}

struct HeaderSkipSeed<'a> {
    depth: usize,
    nodes: &'a Cell<usize>,
}
impl<'de> DeserializeSeed<'de> for HeaderSkipSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        count_header_node(self.nodes)?;
        d.deserialize_any(HeaderSkipVisitor(self))
    }
}
struct HeaderSkipVisitor<'a>(HeaderSkipSeed<'a>);
impl<'de> Visitor<'de> for HeaderSkipVisitor<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("finite source JSON within the whole-record depth and node bounds")
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<(), E> {
        if value.is_finite() {
            Ok(())
        } else {
            Err(E::custom("nonfinite source header number"))
        }
    }
    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<(), E> {
        if text.len() <= MAX_RECORD_BYTES {
            Ok(())
        } else {
            Err(E::custom("source header string byte limit"))
        }
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        if self.0.depth >= MAX_HEADER_CONTAINER_DEPTH {
            return Err(A::Error::custom("source header JSON nesting limit"));
        }
        while seq
            .next_element_seed(HeaderSkipSeed {
                depth: self.0.depth + 1,
                nodes: self.0.nodes,
            })?
            .is_some()
        {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        if self.0.depth >= MAX_HEADER_CONTAINER_DEPTH {
            return Err(A::Error::custom("source header JSON nesting limit"));
        }
        while map.next_key::<DiscardedHeaderText>()?.is_some() {
            map.next_value_seed(HeaderSkipSeed {
                depth: self.0.depth + 1,
                nodes: self.0.nodes,
            })?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct SceneRecord {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) environment_source: Option<String>,
}

// Production retains only the checked identity/edge summary. Detailed decoded
// projections below exist only for parser tests; source bytes remain the authority.
#[derive(Debug)]
pub(crate) struct EnvironmentRecord {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) sun: Sun,
    #[cfg(test)]
    pub(crate) ambient_irradiance_lux: [f64; 3],
    #[cfg(test)]
    pub(crate) background_radiance_cd_m2: [f64; 3],
    #[cfg(test)]
    pub(crate) unsupported: Vec<String>,
    /// Exact raw values, including internal whitespace and number spellings.
    #[cfg(test)]
    pub(crate) extensions: Vec<(String, String)>,
}

#[derive(Debug)]
pub(crate) struct Sun {
    pub(crate) light_ref: String,
    #[cfg(test)]
    pub(crate) toward_source_unit: [f64; 3],
    #[cfg(test)]
    pub(crate) perpendicular_illuminance_lux: f64,
    #[cfg(test)]
    pub(crate) unit_luminance_linear_rec709: [f64; 3],
    #[cfg(test)]
    pub(crate) angular_radius_rad: f64,
}

#[derive(Debug)]
pub(crate) struct LightRecord {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
}

// Every fixed object is decoded directly into typed slots. Unknown/duplicate
// keys fail before their values are decoded; field ordering is immaterial.
macro_rules! strict_object {
    ($name:ident { $($key:literal => $field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Default)]
        struct $name { $( $field: Option<$ty>, )* }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct ObjectVisitor;
                impl<'de> Visitor<'de> for ObjectVisitor {
                    type Value = $name;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str(stringify!($name))
                    }
                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                        let mut result = $name::default();
                        while let Some(key) = map.next_key::<Text>()? {
                            match key.0.as_str() {
                                $($key if result.$field.is_none() => result.$field = Some(map.next_value()?),)*
                                _ => return Err(A::Error::custom("duplicate or unknown record field")),
                            }
                        }
                        Ok(result)
                    }
                }
                d.deserialize_map(ObjectVisitor)
            }
        }
    };
}

strict_object!(SceneFields {
    "scene.label" => label: Text,
    "environment.source" => environment_source: Uuid,
});
strict_object!(LightFields {
    "label" => label: Text,
    "driver_binding" => driver_binding: Text,
});
strict_object!(SunFields {
    "light_ref" => light_ref: Uuid,
    "toward_source_unit" => toward_source_unit: [f64; 3],
    "perpendicular_illuminance_lux" => illuminance: f64,
    "unit_luminance_linear_rec709" => colour: [f64; 3],
    "angular_radius_rad" => angular_radius: f64,
    "direction_model" => direction_model: Text,
    "applied_attenuation" => applied_attenuation: EmptyArray,
});

struct Envelope<T> {
    id: Uuid,
    record_type: Text,
    schema: u64,
    fields: T,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Envelope<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct EnvelopeVisitor<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for EnvelopeVisitor<T> {
            type Value = Envelope<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a fixed engine-owned source envelope")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut id, mut record_type, mut schema, mut fields) = (None, None, None, None);
                let mut documentation_seen = false;
                while let Some(key) = map.next_key::<Text>()? {
                    match key.0.as_str() {
                        "id" if id.is_none() => id = Some(map.next_value()?),
                        "type" if record_type.is_none() => record_type = Some(map.next_value()?),
                        "schema" if schema.is_none() => schema = Some(map.next_value()?),
                        "fields" if fields.is_none() => fields = Some(map.next_value()?),
                        "documentation" if !documentation_seen => {
                            let _: Text = map.next_value()?;
                            documentation_seen = true;
                        }
                        _ => return Err(A::Error::custom("duplicate or unknown envelope key")),
                    }
                }
                Ok(Envelope {
                    id: required(id)?,
                    record_type: required(record_type)?,
                    schema: required(schema)?,
                    fields: required(fields)?,
                })
            }
        }
        d.deserialize_map(EnvelopeVisitor(PhantomData))
    }
}

fn required<T, E: serde::de::Error>(value: Option<T>) -> Result<T, E> {
    value.ok_or_else(|| E::custom("missing required field"))
}

struct Text(String);
impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct TextVisitor;
        impl Visitor<'_> for TextVisitor {
            type Value = Text;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a bounded UTF-8 string")
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Text, E> {
                if s.len() > MAX_STRING_BYTES {
                    return Err(E::custom("string byte limit"));
                }
                Ok(Text(s.to_owned()))
            }
        }
        d.deserialize_str(TextVisitor)
    }
}

struct Uuid(String);
impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = Text::deserialize(d)?;
        if !canonical_uuid(&text.0) {
            return Err(D::Error::custom("noncanonical UUID"));
        }
        Ok(Self(text.0))
    }
}

struct EmptyArray;
impl<'de> Deserialize<'de> for EmptyArray {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct EmptyVisitor;
        impl<'de> Visitor<'de> for EmptyVisitor {
            type Value = EmptyArray;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an empty array")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                // Reject as soon as the first value exists, before asking the
                // deserializer to inspect or traverse that value.
                let _: Option<()> = seq.next_element_seed(RejectAttenuation)?;
                Ok(EmptyArray)
            }
        }
        d.deserialize_seq(EmptyVisitor)
    }
}

struct RejectAttenuation;
impl<'de> DeserializeSeed<'de> for RejectAttenuation {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, _d: D) -> Result<(), D::Error> {
        Err(D::Error::custom(
            "raw source cannot carry applied attenuation",
        ))
    }
}

const UNSUPPORTED: [&str; 7] = [
    "wind",
    "cloud_tau",
    "precipitation",
    "air",
    "thermal",
    "moon",
    "air_quality",
];
struct Unsupported {
    #[cfg(test)]
    values: Vec<String>,
}
impl<'de> Deserialize<'de> for Unsupported {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ListVisitor;
        impl<'de> Visitor<'de> for ListVisitor {
            type Value = Unsupported;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("the constant-light unsupported quantity list")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = Vec::new();
                while let Some(text) = seq.next_element::<Text>()? {
                    if !UNSUPPORTED.contains(&text.0.as_str()) || result.contains(&text.0) {
                        return Err(A::Error::custom(
                            "unknown or duplicate unsupported quantity",
                        ));
                    }
                    result.push(text.0);
                }
                if result.len() != UNSUPPORTED.len() {
                    return Err(A::Error::custom(
                        "incomplete constant-light unsupported list",
                    ));
                }
                Ok(Unsupported {
                    #[cfg(test)]
                    values: result,
                })
            }
        }
        d.deserialize_seq(ListVisitor)
    }
}

#[derive(Default)]
struct EnvironmentFields {
    label: Option<Text>,
    profile: Option<Text>,
    sun: Option<SunFields>,
    ambient: Option<[f64; 3]>,
    background: Option<[f64; 3]>,
    unsupported: Option<Unsupported>,
    extension_keys: Vec<String>,
    #[cfg(test)]
    extensions: Vec<(String, String)>,
}
impl<'de> Deserialize<'de> for EnvironmentFields {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct FieldsVisitor;
        impl<'de> Visitor<'de> for FieldsVisitor {
            type Value = EnvironmentFields;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("constant-light-v1 fields")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = EnvironmentFields::default();
                let mut total_bytes = 0;
                let total_nodes = Cell::new(0);
                while let Some(key) = map.next_key::<Text>()? {
                    match key.0.as_str() {
                        "label" if out.label.is_none() => out.label = Some(map.next_value()?),
                        "profile" if out.profile.is_none() => out.profile = Some(map.next_value()?),
                        "sun" if out.sun.is_none() => out.sun = Some(map.next_value()?),
                        "ambient_irradiance_lux" if out.ambient.is_none() => {
                            out.ambient = Some(map.next_value()?)
                        }
                        "background_radiance_cd_m2" if out.background.is_none() => {
                            out.background = Some(map.next_value()?)
                        }
                        "unsupported" if out.unsupported.is_none() => {
                            out.unsupported = Some(map.next_value()?)
                        }
                        k if k.starts_with("x.thrum.environment.") && k.is_ascii() => {
                            if out.extension_keys.len() >= MAX_EXTENSIONS
                                || out.extension_keys.iter().any(|old| old == k)
                            {
                                return Err(A::Error::custom("duplicate or excess opaque field"));
                            }
                            // Borrow the raw slice first: oversized values are not copied.
                            let raw: &RawValue = map.next_value()?;
                            total_bytes += raw.get().len();
                            if raw.get().len() > MAX_EXTENSION_BYTES
                                || total_bytes > MAX_TOTAL_EXTENSION_BYTES
                            {
                                return Err(A::Error::custom("opaque JSON byte limit"));
                            }
                            check_opaque(raw.get(), &total_nodes).map_err(A::Error::custom)?;
                            #[cfg(test)]
                            out.extensions.push((key.0.clone(), raw.get().to_owned()));
                            out.extension_keys.push(key.0);
                        }
                        _ => {
                            return Err(A::Error::custom("duplicate or unknown environment field"));
                        }
                    }
                }
                Ok(out)
            }
        }
        d.deserialize_map(FieldsVisitor)
    }
}

fn parse<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    kind: &str,
    version: u64,
) -> Result<Envelope<T>, serde_json::Error> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(serde_json::Error::custom("record byte limit"));
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let record = Envelope::<T>::deserialize(&mut d)?;
    d.end()?;
    if record.record_type.0 != kind || record.schema != version {
        return Err(serde_json::Error::custom(
            "unsupported record type or schema",
        ));
    }
    Ok(record)
}

pub(crate) fn parse_scene(bytes: &[u8]) -> Result<SceneRecord, serde_json::Error> {
    let record = parse::<SceneFields>(bytes, "core.scene", 2)?;
    Ok(SceneRecord {
        id: record.id.0,
        label: record.fields.label.map(|x| x.0),
        environment_source: record.fields.environment_source.map(|x| x.0),
    })
}

pub(crate) fn parse_light(bytes: &[u8]) -> Result<LightRecord, serde_json::Error> {
    let record = parse::<LightFields>(bytes, "thrum.light.identity", 1)?;
    if required::<_, serde_json::Error>(record.fields.driver_binding)?.0
        != "incoming-environment-sun"
    {
        return Err(serde_json::Error::custom("unsupported light driver"));
    }
    Ok(LightRecord {
        id: record.id.0,
        label: record.fields.label.map(|x| x.0),
    })
}

pub(crate) fn parse_environment(bytes: &[u8]) -> Result<EnvironmentRecord, serde_json::Error> {
    let record = parse::<EnvironmentFields>(bytes, "thrum.environment.authored", 1)?;
    let fields = record.fields;
    if required::<_, serde_json::Error>(fields.profile)?.0 != "constant-light-v1" {
        return Err(serde_json::Error::custom("unsupported environment profile"));
    }
    let sun = required::<_, serde_json::Error>(fields.sun)?;
    if required::<_, serde_json::Error>(sun.direction_model)?.0 != "geometric-straight" {
        return Err(serde_json::Error::custom("unsupported direction model"));
    }
    required::<_, serde_json::Error>(sun.applied_attenuation)?;
    let direction = required::<_, serde_json::Error>(sun.toward_source_unit)?;
    let colour = required::<_, serde_json::Error>(sun.colour)?;
    let illuminance = required::<_, serde_json::Error>(sun.illuminance)?;
    let radius = required::<_, serde_json::Error>(sun.angular_radius)?;
    let ambient = required::<_, serde_json::Error>(fields.ambient)?;
    let background = required::<_, serde_json::Error>(fields.background)?;
    let _unsupported = required::<_, serde_json::Error>(fields.unsupported)?;
    let length = direction[0].hypot(direction[1]).hypot(direction[2]);
    let luminance = 0.2126 * colour[0] + 0.7152 * colour[1] + 0.0722 * colour[2];
    if direction.iter().any(|x| !x.is_finite())
        || !length.is_finite()
        || length == 0.0
        || (length - 1.0).abs() > UNIT_TOLERANCE
        || !nonnegative(&colour)
        || (luminance - 1.0).abs() > UNIT_TOLERANCE
        || !nonnegative(&[illuminance, radius])
        || !nonnegative(&ambient)
        || !nonnegative(&background)
    {
        return Err(serde_json::Error::custom(
            "invalid finite, nonnegative or normalized light quantity",
        ));
    }
    Ok(EnvironmentRecord {
        id: record.id.0,
        label: fields.label.map(|x| x.0),
        sun: Sun {
            light_ref: required::<_, serde_json::Error>(sun.light_ref)?.0,
            #[cfg(test)]
            toward_source_unit: direction,
            #[cfg(test)]
            perpendicular_illuminance_lux: illuminance,
            #[cfg(test)]
            unit_luminance_linear_rec709: colour,
            #[cfg(test)]
            angular_radius_rad: radius,
        },
        #[cfg(test)]
        ambient_irradiance_lux: ambient,
        #[cfg(test)]
        background_radiance_cd_m2: background,
        #[cfg(test)]
        unsupported: _unsupported.values,
        #[cfg(test)]
        extensions: fields.extensions,
    })
}
fn nonnegative(values: &[f64]) -> bool {
    values.iter().all(|x| x.is_finite() && *x >= 0.0)
}

/// Check the declared namespace and selected source separately from decoding.
/// Declared but unselected sources remain content; they do not activate a Sun.
#[cfg(test)]
pub(crate) fn validate_closure(
    scene: &SceneRecord,
    environments: &[EnvironmentRecord],
    lights: &[LightRecord],
) -> Result<(), &'static str> {
    let mut ids = BTreeSet::new();
    ids.insert(&scene.id);
    for id in environments
        .iter()
        .map(|x| &x.id)
        .chain(lights.iter().map(|x| &x.id))
    {
        if !ids.insert(id) {
            return Err("duplicate declared UUID");
        }
    }
    for source in environments {
        if !lights.iter().any(|light| light.id == source.sun.light_ref) {
            return Err("unresolved or wrong-type light reference");
        }
    }
    if let Some(selected) = &scene.environment_source
        && !environments.iter().any(|source| &source.id == selected)
    {
        return Err("unresolved or wrong-type environment source");
    }
    Ok(())
}

// Validation-only traversal of opaque JSON; no generic value tree is built.
// Depth counts containers, root included. Nodes count values including containers;
// object keys are checked strings but not value nodes. Node count is shared by
// every opaque field; depth and string bounds apply within each raw value.
fn check_opaque(raw: &str, nodes: &Cell<usize>) -> Result<(), serde_json::Error> {
    let mut d = serde_json::Deserializer::from_str(raw);
    OpaqueSeed { depth: 0, nodes }.deserialize(&mut d)?;
    d.end()
}
struct OpaqueSeed<'a> {
    depth: usize,
    nodes: &'a Cell<usize>,
}
impl<'de> DeserializeSeed<'de> for OpaqueSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        let nodes = self.nodes.get();
        if nodes >= MAX_EXTENSION_NODES {
            return Err(D::Error::custom("opaque node limit"));
        }
        self.nodes.set(nodes + 1);
        d.deserialize_any(OpaqueVisitor(self))
    }
}
struct OpaqueVisitor<'a>(OpaqueSeed<'a>);
impl<'de> Visitor<'de> for OpaqueVisitor<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded finite opaque JSON")
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, x: f64) -> Result<(), E> {
        if x.is_finite() {
            Ok(())
        } else {
            Err(E::custom("nonfinite opaque number"))
        }
    }
    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<(), E> {
        if s.len() <= MAX_STRING_BYTES {
            Ok(())
        } else {
            Err(E::custom("opaque string limit"))
        }
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        if self.0.depth >= MAX_EXTENSION_DEPTH {
            return Err(A::Error::custom("opaque depth limit"));
        }
        while seq
            .next_element_seed(OpaqueSeed {
                depth: self.0.depth + 1,
                nodes: self.0.nodes,
            })?
            .is_some()
        {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        if self.0.depth >= MAX_EXTENSION_DEPTH {
            return Err(A::Error::custom("opaque depth limit"));
        }
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<Text>()? {
            if !keys.insert(key.0) {
                return Err(A::Error::custom("duplicate opaque key"));
            }
            map.next_value_seed(OpaqueSeed {
                depth: self.0.depth + 1,
                nodes: self.0.nodes,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &str = "10000000-0000-4000-8000-000000000003";
    const SOURCE: &str = "10000000-0000-4000-8000-000000000001";
    const LIGHT: &str = "10000000-0000-4000-8000-000000000002";
    const ENVIRONMENT: &str = r#"{
      "type":"thrum.environment.authored","schema":1,
      "id":"10000000-0000-4000-8000-000000000001",
      "fields":{
        "profile":"constant-light-v1",
        "sun":{
          "light_ref":"10000000-0000-4000-8000-000000000002",
          "toward_source_unit":[0,1,0],
          "perpendicular_illuminance_lux":100000,
          "unit_luminance_linear_rec709":[1,1,1],
          "angular_radius_rad":0.00465,
          "direction_model":"geometric-straight","applied_attenuation":[]
        },
        "ambient_irradiance_lux":[1000,1000,1000],
        "background_radiance_cd_m2":[0,0,0],
        "unsupported":["wind","cloud_tau","precipitation","air","thermal","moon","air_quality"]
      }
    }"#;

    fn scene(source: Option<&str>) -> SceneRecord {
        let fields = source.map_or_else(
            || "{}".to_owned(),
            |id| format!(r#"{{"scene.label":"entry","environment.source":"{id}"}}"#),
        );
        parse_scene(
            format!(r#"{{"id":"{SCENE}","type":"core.scene","schema":2,"fields":{fields}}}"#)
                .as_bytes(),
        )
        .unwrap()
    }
    fn light() -> LightRecord {
        parse_light(format!(r#"{{"id":"{LIGHT}","type":"thrum.light.identity","schema":1,"fields":{{"label":"Sun","driver_binding":"incoming-environment-sun"}}}}"#).as_bytes()).unwrap()
    }
    fn with_extension(key: &str, raw: &str) -> String {
        ENVIRONMENT.replace(
            r#""profile":"constant-light-v1""#,
            &format!(r#""{key}":{raw},"profile":"constant-light-v1""#),
        )
    }

    #[test]
    fn production_header_counts_the_complete_envelope_and_bounds_depth() {
        let wrap = |fields: &str| {
            format!(
                r#"{{"fields":{fields},"id":"{SOURCE}","type":"thrum.environment.authored","schema":1}}"#
            )
        };
        // The depth-0 envelope consumes one of the 64 permitted containers,
        // leaving 63 nested array containers under its fields value.
        for (nested_arrays, accepted) in [(63, true), (64, false)] {
            let fields = format!(
                "{}0{}",
                "[".repeat(nested_arrays),
                "]".repeat(nested_arrays)
            );
            assert_eq!(record_header(wrap(&fields).as_bytes()).is_ok(), accepted);
        }
        // Root + fields array + id/type/schema = five non-element values.
        for (elements, accepted) in [(MAX_RECORD_NODES - 5, true), (MAX_RECORD_NODES - 4, false)] {
            let fields = format!("[{}0]", "0,".repeat(elements - 1));
            assert_eq!(record_header(wrap(&fields).as_bytes()).is_ok(), accepted);
        }
        // Dispatch is order-independent, and a nested type does not select a
        // different parser. Nested key semantics remain the full parser's job.
        let header =
            record_header(wrap(r#"{"type":"game.fake","same":0,"same":1}"#).as_bytes()).unwrap();
        assert_eq!(header.record_type, "thrum.environment.authored");
        assert_eq!(header.schema, 1);
        let documented = wrap("{}").replacen(
            '{',
            &format!(r#"{{"documentation":"{}","#, "a".repeat(8192)),
            1,
        );
        assert!(record_header(documented.as_bytes()).is_ok());
    }

    #[test]
    fn exact_authored_fixture_decodes_without_weather_defaults() {
        let source = parse_environment(ENVIRONMENT.as_bytes()).unwrap();
        assert_eq!(source.id, SOURCE);
        assert!(source.label.is_none());
        assert_eq!(source.sun.toward_source_unit, [0.0, 1.0, 0.0]);
        assert_eq!(source.sun.perpendicular_illuminance_lux, 100000.0);
        assert_eq!(source.sun.unit_luminance_linear_rec709, [1.0; 3]);
        assert_eq!(source.sun.angular_radius_rad, 0.00465);
        assert_eq!(source.ambient_irradiance_lux, [1000.0; 3]);
        assert_eq!(source.background_radiance_cd_m2, [0.0; 3]);
        assert_eq!(source.unsupported, UNSUPPORTED);
        assert!(source.extensions.is_empty());
        let selected = scene(Some(SOURCE));
        let light = light();
        assert_eq!(selected.label.as_deref(), Some("entry"));
        assert_eq!(light.label.as_deref(), Some("Sun"));
        assert!(validate_closure(&selected, &[source], &[light]).is_ok());
        // Absence of a binding is legal, but does not activate declared content.
        assert!(scene(None).environment_source.is_none());
    }

    #[test]
    fn optional_envelope_documentation_is_bounded_decoded_text_for_all_record_kinds() {
        let records = [
            format!(r#"{{"id":"{SCENE}","type":"core.scene","schema":2,"fields":{{}}}}"#),
            ENVIRONMENT.to_owned(),
            format!(
                r#"{{"id":"{LIGHT}","type":"thrum.light.identity","schema":1,"fields":{{"driver_binding":"incoming-environment-sun"}}}}"#
            ),
        ];
        for (kind, record) in records.iter().enumerate() {
            let accepts = |bytes: &[u8]| match kind {
                0 => parse_scene(bytes).is_ok(),
                1 => parse_environment(bytes).is_ok(),
                _ => parse_light(bytes).is_ok(),
            };
            for text in [
                String::new(),
                "a".repeat(MAX_STRING_BYTES),
                "\\u00e9".repeat(MAX_STRING_BYTES / 2),
            ] {
                let documented =
                    record.replacen('{', &format!(r#"{{"documentation":"{text}","#), 1);
                assert!(accepts(documented.as_bytes()));
            }
            for value in [
                format!("\"{}\"", "a".repeat(MAX_STRING_BYTES + 1)),
                format!("\"{}\"", "\\u00e9".repeat(MAX_STRING_BYTES / 2 + 1)),
                "null".to_owned(),
                "0".to_owned(),
                "{}".to_owned(),
            ] {
                let invalid = record.replacen('{', &format!(r#"{{"documentation":{value},"#), 1);
                assert!(!accepts(invalid.as_bytes()));
            }
            let duplicate =
                record.replacen('{', r#"{"documentation":"a","document\u0061tion":"b","#, 1);
            assert!(!accepts(duplicate.as_bytes()));
        }
    }

    #[test]
    fn nonempty_attenuation_refuses_before_reading_the_first_value() {
        // Both malformed and deeply nested first values would produce different
        // errors if they were traversed before the nonempty-array refusal.
        for first in [
            "{\"unterminated\":".to_owned(),
            format!("{}0{}", "[".repeat(1000), "]".repeat(1000)),
            format!("\"{}\"", "x".repeat(64 * 1024)),
        ] {
            let record = ENVIRONMENT.replace(
                "\"applied_attenuation\":[]",
                &format!("\"applied_attenuation\":[{first}]"),
            );
            let error = parse_environment(record.as_bytes()).unwrap_err();
            assert!(
                error
                    .to_string()
                    .starts_with("raw source cannot carry applied attenuation")
            );
        }
    }

    #[test]
    fn rejects_wrong_type_version_uuid_shape_and_missing_quantities() {
        for invalid in [
            ENVIRONMENT.replace("thrum.environment.authored", "game.weather"),
            ENVIRONMENT.replace("\"schema\":1", "\"schema\":2"),
            ENVIRONMENT.replace("constant-light-v1", "authored-vertical-rain-v1"),
            ENVIRONMENT.replace(SOURCE, "10000000-0000-0000-8000-000000000001"),
            ENVIRONMENT.replace(":100000", ":\"100000\""),
            ENVIRONMENT.replace("[0,1,0]", "[0,1]"),
            ENVIRONMENT.replace("\"ambient_irradiance_lux\":[1000,1000,1000],", ""),
            ENVIRONMENT.replace("\"air_quality\"", "\"AIR_QUALITY\""),
            ENVIRONMENT.replace(",\"air_quality\"", ""),
            ENVIRONMENT.replace("\"moon\",\"air_quality\"", "\"moon\",\"moon\""),
            format!("{ENVIRONMENT} trailing"),
        ] {
            assert!(parse_environment(invalid.as_bytes()).is_err(), "{invalid}");
        }
        assert!(
            parse_scene(
                format!(r#"{{"id":"{SCENE}","type":"core.scene","schema":1,"fields":{{}}}}"#)
                    .as_bytes()
            )
            .is_err()
        );
        assert!(parse_scene(format!(r#"{{"id":"{SCENE}","type":"core.scene","schema":2,"fields":{{"environment.source":null}}}}"#).as_bytes()).is_err());
        assert!(parse_light(format!(r#"{{"id":"{LIGHT}","type":"thrum.light.identity","schema":1,"fields":{{"driver_binding":"independent-transform"}}}}"#).as_bytes()).is_err());
    }

    #[test]
    fn rejects_duplicates_at_each_object_level_and_escaped_aliases() {
        for invalid in [
            ENVIRONMENT.replacen("\"schema\":1", "\"schema\":1,\"schema\":1", 1),
            ENVIRONMENT.replace(
                "\"profile\":",
                "\"profile\":\"constant-light-v1\",\"profile\":",
            ),
            ENVIRONMENT.replace(
                "\"light_ref\":",
                &format!("\"light_ref\":\"{LIGHT}\",\"light_ref\":"),
            ),
            with_extension("x.thrum.environment.test", r#"{"a":1,"\u0061":2}"#),
            with_extension("x.thrum.environment.test", r#"{"nested":{"a":1,"a":2}}"#),
            with_extension("x.thrum.environment.test", "0").replace(
                "\"profile\":",
                "\"x.thrum.environment.test\":1,\"profile\":",
            ),
        ] {
            assert!(parse_environment(invalid.as_bytes()).is_err());
        }
        let duplicate_scene = format!(
            r#"{{"id":"{SCENE}","type":"core.scene","schema":2,"fields":{{"environment.source":"{SOURCE}","environment.source":"{SOURCE}"}}}}"#
        );
        assert!(parse_scene(duplicate_scene.as_bytes()).is_err());
        let duplicate_light = format!(
            r#"{{"id":"{LIGHT}","type":"thrum.light.identity","schema":1,"fields":{{"label":"a","label":"b","driver_binding":"incoming-environment-sun"}}}}"#
        );
        assert!(parse_light(duplicate_light.as_bytes()).is_err());
    }

    #[test]
    fn refuses_invented_weather_fields_and_rival_light_ownership() {
        for field in [
            "wind",
            "cloud_tau",
            "precipitation",
            "air",
            "thermal",
            "moon",
            "air_quality",
            "temperature",
            "wetness",
        ] {
            assert!(parse_environment(with_extension(field, "0").as_bytes()).is_err());
        }
        assert!(
            parse_environment(
                ENVIRONMENT
                    .replace("\"sun\":{", "\"sun\":{\"x.thrum.environment.test\":0,")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_environment(
                ENVIRONMENT
                    .replace(
                        "\"applied_attenuation\":[]",
                        "\"applied_attenuation\":[\"cloud\"]"
                    )
                    .as_bytes()
            )
            .is_err()
        );
        let rival = format!(
            r#"{{"id":"{LIGHT}","type":"thrum.light.identity","schema":1,"fields":{{"driver_binding":"incoming-environment-sun","transform.direction":[0,1,0]}}}}"#
        );
        assert!(parse_light(rival.as_bytes()).is_err());
    }

    #[test]
    fn enforces_finite_nonnegative_and_authored_normalization_bounds() {
        for invalid in [
            ENVIRONMENT.replace(":100000", ":-1"),
            ENVIRONMENT.replace(":100000", ":1e999"),
            ENVIRONMENT.replace(":100000", ":NaN"),
            ENVIRONMENT.replace("0.00465", "-0.1"),
            ENVIRONMENT.replace("[0,1,0]", "[0,0,0]"),
            ENVIRONMENT.replace("[0,1,0]", "[0,1.0000000011,0]"),
            ENVIRONMENT.replace("[1,1,1]", "[1.0000000011,1.0000000011,1.0000000011]"),
            ENVIRONMENT.replace("[1,1,1]", "[-1,1,1]"),
            ENVIRONMENT.replace("[1000,1000,1000]", "[-1,0,0]"),
            ENVIRONMENT.replace("[0,0,0]", "[0,-1,0]"),
        ] {
            assert!(parse_environment(invalid.as_bytes()).is_err(), "{invalid}");
        }
        let non_cardinal = ENVIRONMENT.replace("[0,1,0]", "[0.6,0.8,0]");
        assert_eq!(
            parse_environment(non_cardinal.as_bytes())
                .unwrap()
                .sun
                .toward_source_unit,
            [0.6, 0.8, 0.0]
        );
        assert!(
            parse_environment(
                ENVIRONMENT
                    .replace("[0,1,0]", "[0,1.0000000009,0]")
                    .as_bytes()
            )
            .is_ok()
        );
        assert!(
            parse_environment(
                ENVIRONMENT
                    .replace("[1,1,1]", "[1.0000000009,1.0000000009,1.0000000009]")
                    .as_bytes()
            )
            .is_ok()
        );
    }

    #[test]
    fn closure_checks_selected_source_typed_links_and_global_id_collisions() {
        let source = || parse_environment(ENVIRONMENT.as_bytes()).unwrap();
        assert_eq!(
            validate_closure(&scene(Some(LIGHT)), &[source()], &[light()]),
            Err("unresolved or wrong-type environment source")
        );
        assert_eq!(
            validate_closure(&scene(Some(SOURCE)), &[source()], &[]),
            Err("unresolved or wrong-type light reference")
        );
        assert_eq!(
            validate_closure(&scene(Some(SOURCE)), &[source(), source()], &[light()]),
            Err("duplicate declared UUID")
        );
        let mut wrong_type = source();
        wrong_type.sun.light_ref = SCENE.to_owned();
        assert_eq!(
            validate_closure(&scene(Some(SOURCE)), &[wrong_type], &[light()]),
            Err("unresolved or wrong-type light reference")
        );
        let mut collision = light();
        collision.id = SCENE.to_owned();
        assert_eq!(
            validate_closure(&scene(None), &[], &[collision]),
            Err("duplicate declared UUID")
        );
        assert!(validate_closure(&scene(None), &[source()], &[light()]).is_ok());
    }

    #[test]
    fn opaque_values_preserve_bytes_during_neighboring_known_edit() {
        let raw = r#"{ "nested" : [1.00, -0, "\u0061", null, true, false], "x": {} }"#;
        let original = with_extension("x.thrum.environment.test", raw);
        let edited = original.replace(":100000", ":75000");
        let before = parse_environment(original.as_bytes()).unwrap();
        let after = parse_environment(edited.as_bytes()).unwrap();
        assert_eq!(before.extensions, after.extensions);
        assert_eq!(
            after.extensions,
            [("x.thrum.environment.test".to_owned(), raw.to_owned())]
        );
        assert_eq!(after.sun.perpendicular_illuminance_lux, 75000.0);
    }

    #[test]
    fn opaque_byte_count_depth_nodes_and_string_limits_have_boundaries() {
        let prefix = "x.thrum.environment.test";
        let at_bytes = format!("[{}0]", " ".repeat(MAX_EXTENSION_BYTES - 3));
        assert_eq!(at_bytes.len(), MAX_EXTENSION_BYTES);
        assert!(parse_environment(with_extension(prefix, &at_bytes).as_bytes()).is_ok());
        let too_many_bytes = format!("[{}0]", " ".repeat(MAX_EXTENSION_BYTES - 2));
        assert!(parse_environment(with_extension(prefix, &too_many_bytes).as_bytes()).is_err());
        for (count, accepted) in [(MAX_EXTENSIONS, true), (MAX_EXTENSIONS + 1, false)] {
            let fields: String = (0..count)
                .map(|i| format!("\"{prefix}{i}\":{at_bytes},"))
                .collect();
            let record = ENVIRONMENT.replace("\"profile\":", &format!("{fields}\"profile\":"));
            assert_eq!(parse_environment(record.as_bytes()).is_ok(), accepted);
        }
        for (depth, accepted) in [(16, true), (17, false)] {
            let raw = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
            assert_eq!(
                parse_environment(with_extension(prefix, &raw).as_bytes()).is_ok(),
                accepted
            );
        }
        for (nodes, accepted) in [(4096, true), (4097, false)] {
            let raw = format!("[{}0]", "0,".repeat(nodes - 2));
            assert_eq!(
                parse_environment(with_extension(prefix, &raw).as_bytes()).is_ok(),
                accepted
            );
        }
        // Two individually legal values still share the aggregate node budget.
        for (second_nodes, accepted) in [(2048, true), (2049, false)] {
            let first = format!("[{}0]", "0,".repeat(2046));
            let second = format!("[{}0]", "0,".repeat(second_nodes - 2));
            let record = with_extension(prefix, &first).replace(
                "\"profile\":",
                &format!("\"x.thrum.environment.second\":{second},\"profile\":"),
            );
            assert_eq!(parse_environment(record.as_bytes()).is_ok(), accepted);
        }
        for (len, accepted) in [(4096, true), (4097, false)] {
            let text = format!("\"{}\"", "a".repeat(len));
            assert_eq!(
                parse_environment(with_extension(prefix, &text).as_bytes()).is_ok(),
                accepted
            );
            let key = format!("{{{text}:0}}");
            assert_eq!(
                parse_environment(with_extension(prefix, &key).as_bytes()).is_ok(),
                accepted
            );
        }
        assert!(parse_environment(with_extension(prefix, r#""\ud800""#).as_bytes()).is_err());
        assert!(parse_environment(with_extension(prefix, "1e999").as_bytes()).is_err());
        assert!(parse_environment(&vec![b' '; MAX_RECORD_BYTES + 1]).is_err());
    }
}
