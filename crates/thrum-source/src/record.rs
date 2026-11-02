//! Common identity envelope for editable engine JSON records.
//!
//! F3's schema registry must additionally validate known field meanings, units,
//! required references, nested IDs and migrations before source admission.

use serde_json::Value;

/// Validated stable identity and schema version of one source record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordEnvelope<'a> {
    /// Canonical lowercase RFC-4122-form UUID.
    id: &'a str,
    /// Nonempty ASCII schema type ID; F3 owns its registered spelling.
    record_type: &'a str,
    /// Positive schema version.
    schema: u64,
}

impl<'a> RecordEnvelope<'a> {
    /// Canonical stable record ID.
    pub fn id(&self) -> &'a str {
        self.id
    }

    /// Nonempty ASCII type ID for F3 registry lookup.
    pub fn record_type(&self) -> &'a str {
        self.record_type
    }

    /// Positive source schema version.
    pub fn schema(&self) -> u64 {
        self.schema
    }
}

/// A record does not have the common F3 identity envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordError {
    /// The record is not an object with the required fields.
    Shape,
    /// The stable ID is not a canonical lowercase RFC-4122-form UUID.
    Id,
    /// The type ID is empty or non-ASCII.
    Type,
    /// The schema version is not a positive integer.
    Schema,
    /// A field ID is empty or non-ASCII.
    FieldId,
}

/// Validate the common identity fields without claiming semantic schema validity.
///
/// `parse_record_json` must be called first to reject duplicate JSON keys and
/// unbounded input. The F3 registry then checks fields, dependencies and IDs in
/// nested records before a captured source revision can be admitted.
pub fn validate_record_envelope(value: &Value) -> Result<RecordEnvelope<'_>, RecordError> {
    let object = value.as_object().ok_or(RecordError::Shape)?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(RecordError::Shape)?;
    if !canonical_uuid(id) {
        return Err(RecordError::Id);
    }
    let record_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or(RecordError::Shape)?;
    if !nonempty_ascii(record_type) {
        return Err(RecordError::Type);
    }
    let schema = object
        .get("schema")
        .and_then(Value::as_u64)
        .filter(|schema| *schema > 0)
        .ok_or(RecordError::Schema)?;
    let fields = object
        .get("fields")
        .and_then(Value::as_object)
        .ok_or(RecordError::Shape)?;
    if fields.keys().any(|id| !nonempty_ascii(id)) {
        return Err(RecordError::FieldId);
    }
    Ok(RecordEnvelope {
        id,
        record_type,
        schema,
    })
}

fn nonempty_ascii(id: &str) -> bool {
    !id.is_empty() && id.is_ascii()
}

pub(crate) fn canonical_uuid(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (index, byte) in bytes.iter().copied().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return false;
            }
        } else if !matches!(byte, b'0'..=b'9' | b'a'..=b'f') {
            return false;
        }
    }
    // RFC-4122 layout requires a nonzero version nibble and the 10xx variant.
    matches!(bytes[14], b'1'..=b'8') && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_record_json;

    #[test]
    fn common_record_identity_is_checked_before_schema_validation() {
        let valid = br#"{"id":"2a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene-object","schema":1,"fields":{"transform.translation":[2,0,4]}}"#;
        let value = parse_record_json(valid).unwrap();
        let identity = validate_record_envelope(&value).unwrap();
        assert_eq!(identity.record_type(), "core.scene-object");
        assert_eq!(identity.schema(), 1);

        for invalid in [
            br#"{"id":"2A5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene-object","schema":1,"fields":{}}"#.as_slice(),
            br#"{"id":"2a5ef644-7225-4770-1f4d-e1d9c0a7d515","type":"core.scene-object","schema":1,"fields":{}}"#,
            br#"{"id":"2a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"caf\u00e9.scene-object","schema":1,"fields":{}}"#,
            br#"{"id":"2a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene-object","schema":0,"fields":{}}"#,
            br#"{"id":"2a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene-object","schema":1,"fields":{"transform.\u00e9":0}}"#,
        ] {
            let value = parse_record_json(invalid).unwrap();
            assert!(validate_record_envelope(&value).is_err());
        }
    }
}
