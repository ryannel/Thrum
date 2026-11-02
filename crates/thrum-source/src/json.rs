//! Bounded syntax parsing for editable source JSON records.

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

/// Maximum exact bytes in one editable JSON source record.
pub const MAX_RECORD_BYTES: usize = 16 * 1024 * 1024;
const MAX_CONTAINER_DEPTH: u8 = 64;

/// A source JSON record exceeded its byte bound or failed strict syntax.
#[derive(Debug)]
pub enum JsonError {
    /// The exact input exceeded the selected 16 MiB record bound.
    TooLarge,
    /// Syntax, duplicate-key, numeric or nesting validation failed.
    Invalid(serde_json::Error),
}

/// Parse one bounded source record, rejecting duplicate keys at every depth.
/// Schema identity, field units and dependency closure are checked by the
/// later F3 validator before a candidate revision can be returned.
pub fn parse_record_json(bytes: &[u8]) -> Result<Value, JsonError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(JsonError::TooLarge);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictSeed { depth: 0 }
        .deserialize(&mut deserializer)
        .map_err(JsonError::Invalid)?;
    deserializer.end().map_err(JsonError::Invalid)?;
    Ok(value)
}

struct StrictSeed {
    depth: u8,
}

impl<'de> DeserializeSeed<'de> for StrictSeed {
    type Value = Value;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(StrictVisitor { depth: self.depth })
    }
}

struct StrictVisitor {
    depth: u8,
}

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a finite JSON value with unique object keys and bounded nesting")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("nonfinite source number"))
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        if self.depth >= MAX_CONTAINER_DEPTH {
            return Err(serde::de::Error::custom("source JSON nesting limit"));
        }
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictSeed {
            depth: self.depth + 1,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        if self.depth >= MAX_CONTAINER_DEPTH {
            return Err(serde::de::Error::custom("source JSON nesting limit"));
        }
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(StrictSeed {
                depth: self.depth + 1,
            })?;
            if values.insert(key, value).is_some() {
                return Err(serde::de::Error::custom("duplicate source JSON key"));
            }
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ambiguous_and_unbounded_records() {
        assert!(parse_record_json(br#"{"id":1,"id":2}"#).is_err());
        assert!(parse_record_json(br#"{"fields":{"x":1,"x":2}}"#).is_err());
        assert!(parse_record_json(br#"{"x":1e999}"#).is_err());
        assert!(parse_record_json(br#"{"x":1} trailing"#).is_err());
        let deep = format!("{}0{}", "[".repeat(65), "]".repeat(65));
        assert!(parse_record_json(deep.as_bytes()).is_err());
        assert!(matches!(
            parse_record_json(&vec![b' '; MAX_RECORD_BYTES + 1]),
            Err(JsonError::TooLarge)
        ));
        let valid = parse_record_json(br#"{"fields":{"x":1},"id":"a"}"#).unwrap();
        assert_eq!(valid["fields"]["x"], 1);
    }
}
