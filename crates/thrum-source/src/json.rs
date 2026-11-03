//! Bounded syntax parsing for editable source JSON records.

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::cell::Cell;

/// Maximum exact bytes in one editable JSON source record.
pub const MAX_RECORD_BYTES: usize = 16 * 1024 * 1024;
/// Maximum JSON values in one v1 source record, including containers.
pub const MAX_RECORD_NODES: usize = 1_000_000;
pub(crate) const MAX_CONTAINER_DEPTH: u8 = 64;

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
    parse_record_json_with_limit(bytes, MAX_RECORD_NODES)
}

fn parse_record_json_with_limit(bytes: &[u8], node_limit: usize) -> Result<Value, JsonError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(JsonError::TooLarge);
    }
    let nodes = Cell::new(0);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictSeed {
        depth: 0,
        nodes: &nodes,
        node_limit,
    }
    .deserialize(&mut deserializer)
    .map_err(JsonError::Invalid)?;
    deserializer.end().map_err(JsonError::Invalid)?;
    Ok(value)
}

struct StrictSeed<'a> {
    depth: u8,
    nodes: &'a Cell<usize>,
    node_limit: usize,
}

impl<'de> DeserializeSeed<'de> for StrictSeed<'_> {
    type Value = Value;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        let count = self.nodes.get();
        if count >= self.node_limit {
            return Err(serde::de::Error::custom("source JSON value count limit"));
        }
        self.nodes.set(count + 1);
        deserializer.deserialize_any(StrictVisitor {
            depth: self.depth,
            nodes: self.nodes,
            node_limit: self.node_limit,
        })
    }
}

struct StrictVisitor<'a> {
    depth: u8,
    nodes: &'a Cell<usize>,
    node_limit: usize,
}

impl<'de> Visitor<'de> for StrictVisitor<'_> {
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
            nodes: self.nodes,
            node_limit: self.node_limit,
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
                nodes: self.nodes,
                node_limit: self.node_limit,
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
        let precise = parse_record_json(br#"{"value":51.248178375505404}"#).unwrap();
        assert_eq!(precise["value"].as_f64(), Some(51.248178375505404));
        assert!(parse_record_json_with_limit(br#"[0,1]"#, 2).is_err());
        assert!(parse_record_json_with_limit(br#"{"a":0,"b":1}"#, 2).is_err());
        assert!(parse_record_json_with_limit(br#"[0,1]"#, 3).is_ok());
    }

    #[test]
    #[ignore = "local dense-array parser time and process-memory probe"]
    fn measure_dense_array_at_value_limit() {
        let mut bytes = Vec::with_capacity(2 * MAX_RECORD_NODES);
        bytes.push(b'[');
        for index in 0..MAX_RECORD_NODES - 1 {
            if index != 0 {
                bytes.push(b',');
            }
            bytes.push(b'0');
        }
        bytes.push(b']');
        let start = std::time::Instant::now();
        let value = parse_record_json(&bytes).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(value.as_array().unwrap().len(), MAX_RECORD_NODES - 1);
        eprintln!(
            "dense_array_values={} input_bytes={} elapsed={elapsed:?} value_size={}",
            MAX_RECORD_NODES,
            bytes.len(),
            std::mem::size_of::<Value>()
        );
        std::hint::black_box(value);
    }

    #[test]
    #[ignore = "local dense-map parser time and process-memory probe"]
    fn measure_dense_map_at_value_limit() {
        let mut bytes = Vec::with_capacity(14 * MAX_RECORD_NODES);
        bytes.push(b'{');
        for index in 0..MAX_RECORD_NODES - 1 {
            if index != 0 {
                bytes.push(b',');
            }
            bytes.push(b'"');
            bytes.push(b'k');
            bytes.extend_from_slice(index.to_string().as_bytes());
            bytes.extend_from_slice(b"\":0");
        }
        bytes.push(b'}');
        let start = std::time::Instant::now();
        let value = parse_record_json(&bytes).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(value.as_object().unwrap().len(), MAX_RECORD_NODES - 1);
        eprintln!(
            "dense_map_values={} input_bytes={} elapsed={elapsed:?} value_size={}",
            MAX_RECORD_NODES,
            bytes.len(),
            std::mem::size_of::<Value>()
        );
        std::hint::black_box(value);
    }
}
