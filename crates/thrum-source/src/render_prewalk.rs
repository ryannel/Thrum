//! Tree-free bounded JSON shape pass for the inactive render-source decoder.
//!
//! This checks the shared byte/node/depth envelope before typed object work.
//! It intentionally does not validate field keys or semantic values: the
//! subsequent strict typed grammar owns decoded duplicate/unknown keys.
//! Escaped JSON scalar strings can still require serde-owned scratch bounded
//! by the record byte limit; this is not a zero-allocation claim.

use std::cell::Cell;
use std::fmt;

use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::json::{
    JsonError, MAX_CONTAINER_DEPTH, MAX_RECORD_BYTES, MAX_RECORD_NODES, reject_excess_value,
};
use crate::{LimitExceeded, ObservedQuantity, SourceDimension};

pub(crate) fn prewalk_render_json(bytes: &[u8]) -> Result<usize, JsonError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(JsonError::TooLarge {
            observed: bytes.len() as u64,
        });
    }
    let nodes = Cell::new(0);
    let limit = Cell::new(None);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    NoTreeSeed {
        nodes: &nodes,
        limit: &limit,
        depth: 0,
    }
    .deserialize(&mut deserializer)
    .map_err(|source| match limit.get() {
        Some(detail) => JsonError::Limit { detail, source },
        None => JsonError::Invalid(source),
    })?;
    deserializer.end().map_err(JsonError::Invalid)?;
    Ok(nodes.get())
}

struct NoTreeSeed<'a> {
    nodes: &'a Cell<usize>,
    limit: &'a Cell<Option<LimitExceeded>>,
    depth: u8,
}

impl<'de> DeserializeSeed<'de> for NoTreeSeed<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        let count = self.nodes.get();
        if count >= MAX_RECORD_NODES {
            return reject_excess_value(
                deserializer,
                self.limit,
                LimitExceeded {
                    dimension: SourceDimension::GenericJsonNodes,
                    allowed: MAX_RECORD_NODES as u64,
                    observed: ObservedQuantity::AtLeast(count as u64 + 1),
                },
                "render JSON value count limit",
            );
        }
        self.nodes.set(count + 1);
        deserializer.deserialize_any(NoTreeVisitor {
            nodes: self.nodes,
            limit: self.limit,
            depth: self.depth,
        })
    }
}

struct NoTreeVisitor<'a> {
    nodes: &'a Cell<usize>,
    limit: &'a Cell<Option<LimitExceeded>>,
    depth: u8,
}

impl NoTreeVisitor<'_> {
    fn check_container_depth<E: serde::de::Error>(&self) -> Result<(), E> {
        if self.depth >= MAX_CONTAINER_DEPTH {
            self.limit.set(Some(LimitExceeded {
                dimension: SourceDimension::GenericJsonDepth,
                allowed: MAX_CONTAINER_DEPTH as u64,
                observed: ObservedQuantity::AtLeast(self.depth as u64 + 1),
            }));
            return Err(E::custom("render JSON nesting limit"));
        }
        Ok(())
    }
}

impl<'de> Visitor<'de> for NoTreeVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a bounded JSON value")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        if value.is_finite() {
            Ok(())
        } else {
            Err(E::custom("nonfinite render number"))
        }
    }

    fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_string<E: serde::de::Error>(self, _: String) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        self.check_container_depth()?;
        while seq
            .next_element_seed(NoTreeSeed {
                nodes: self.nodes,
                limit: self.limit,
                depth: self.depth + 1,
            })?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        self.check_container_depth()?;
        while map.next_key::<IgnoredAny>()?.is_some() {
            map.next_value_seed(NoTreeSeed {
                nodes: self.nodes,
                limit: self.limit,
                depth: self.depth + 1,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_shared_node_and_depth_boundary_without_a_value_tree() {
        let mut exact = String::with_capacity(2 * MAX_RECORD_NODES);
        exact.push('[');
        exact.push_str(&"0,".repeat(MAX_RECORD_NODES - 2));
        exact.push('0');
        exact.push(']');
        assert_eq!(
            prewalk_render_json(exact.as_bytes()).unwrap(),
            MAX_RECORD_NODES
        );
        exact.insert(exact.len() - 1, ',');
        exact.insert(exact.len() - 1, '0');
        assert!(matches!(
            prewalk_render_json(exact.as_bytes()),
            Err(JsonError::Limit { detail, .. })
                if detail.dimension == SourceDimension::GenericJsonNodes
                    && detail.observed == ObservedQuantity::AtLeast(1_000_001)
        ));

        let at_depth = format!("{}0{}", "[".repeat(64), "]".repeat(64));
        assert!(prewalk_render_json(at_depth.as_bytes()).is_ok());
        let beyond = format!("{}0{}", "[".repeat(65), "]".repeat(65));
        assert!(matches!(
            prewalk_render_json(beyond.as_bytes()),
            Err(JsonError::Limit { detail, .. })
                if detail.dimension == SourceDimension::GenericJsonDepth
        ));
    }

    #[test]
    fn syntax_failure_precedes_semantic_decoding_and_byte_limit_is_exact() {
        assert_eq!(
            prewalk_render_json(br#"{"fields":[{"id":"\u0061"}]}"#).unwrap(),
            4
        );
        assert!(matches!(
            prewalk_render_json(b"[1,2] trailing"),
            Err(JsonError::Invalid(_))
        ));
        assert!(matches!(
            prewalk_render_json(b"[1e999]"),
            Err(JsonError::Invalid(_))
        ));
        assert!(matches!(
            prewalk_render_json(&vec![b' '; MAX_RECORD_BYTES + 1]),
            Err(JsonError::TooLarge { observed }) if observed == (MAX_RECORD_BYTES + 1) as u64
        ));
    }
}
