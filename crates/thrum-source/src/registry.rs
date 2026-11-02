//! Local consistency of parsed first-profile descriptors and source history.
//!
//! This pure join has no source-path, staged-byte or admitted-root authority.

use crate::history_json::RegistryHistory;
use crate::schema_json::SchemaDescriptor;

/// Parsed active descriptors disagree with the source-carried registry history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryConsistencyError {
    /// Active descriptors are not strictly sorted by type ID.
    DescriptorOrder(usize),
    /// Active field at the given descriptor/field index is absent from history.
    MissingField(usize, usize),
    /// History entry at this index is retired but its field is active.
    RetiredActiveField(usize),
    /// History entry at this index disagrees with the active field signature.
    ChangedFieldMeaning(usize),
    /// A nonretired history entry at this index has no active descriptor field.
    OrphanActiveField(usize),
    /// Active descriptor at this index has no matching version-history entry.
    MissingVersion(usize),
    /// Version-history entry at this index disagrees with the active definition.
    ChangedVersionDefinition(usize),
}

/// Check a source registry's active descriptors against its own history.
///
/// Descriptors must be sorted by type ID with at most one current version per
/// type. Each descriptor's fields and the parsed history are already sorted.
/// The two merge joins allocate nothing and take O(active fields + history)
/// time. This does not compare with a prior admitted root.
pub fn validate_active_registry(
    descriptors: &[SchemaDescriptor],
    history: &RegistryHistory<'_>,
) -> Result<(), RegistryConsistencyError> {
    validate_active_registry_with(descriptors, history, |descriptor| descriptor)
}

pub(crate) fn validate_active_registry_with<T, F>(
    descriptors: &[T],
    history: &RegistryHistory<'_>,
    select: F,
) -> Result<(), RegistryConsistencyError>
where
    F: Fn(&T) -> &SchemaDescriptor,
{
    for (index, pair) in descriptors.windows(2).enumerate() {
        if select(&pair[0]).target_type() >= select(&pair[1]).target_type() {
            return Err(RegistryConsistencyError::DescriptorOrder(index + 1));
        }
    }

    let history_fields = history.fields();
    let mut history_index = 0;
    for (descriptor_index, item) in descriptors.iter().enumerate() {
        let descriptor = select(item);
        for (field_index, field) in descriptor.fields().iter().enumerate() {
            let active_key = (descriptor.target_type(), field.id());
            while let Some(entry) = history_fields.get(history_index) {
                let history_key = (entry.type_id(), entry.field_id());
                if history_key >= active_key {
                    break;
                }
                if !entry.retired() {
                    return Err(RegistryConsistencyError::OrphanActiveField(history_index));
                }
                history_index += 1;
            }
            let Some(entry) = history_fields.get(history_index) else {
                return Err(RegistryConsistencyError::MissingField(
                    descriptor_index,
                    field_index,
                ));
            };
            if (entry.type_id(), entry.field_id()) != active_key {
                return Err(RegistryConsistencyError::MissingField(
                    descriptor_index,
                    field_index,
                ));
            }
            if entry.retired() {
                return Err(RegistryConsistencyError::RetiredActiveField(history_index));
            }
            if entry.signature() != field.signature() {
                return Err(RegistryConsistencyError::ChangedFieldMeaning(history_index));
            }
            history_index += 1;
        }
    }
    for (index, entry) in history_fields.iter().enumerate().skip(history_index) {
        if !entry.retired() {
            return Err(RegistryConsistencyError::OrphanActiveField(index));
        }
    }

    let history_versions = history.versions();
    let mut version_index = 0;
    for (descriptor_index, item) in descriptors.iter().enumerate() {
        let descriptor = select(item);
        let active_key = (descriptor.target_type(), descriptor.target_version());
        while history_versions
            .get(version_index)
            .is_some_and(|entry| (entry.type_id(), entry.version()) < active_key)
        {
            version_index += 1;
        }
        let Some(entry) = history_versions.get(version_index) else {
            return Err(RegistryConsistencyError::MissingVersion(descriptor_index));
        };
        if (entry.type_id(), entry.version()) != active_key {
            return Err(RegistryConsistencyError::MissingVersion(descriptor_index));
        }
        if entry.definition_hash() != descriptor.definition_hash() {
            return Err(RegistryConsistencyError::ChangedVersionDefinition(
                version_index,
            ));
        }
        version_index += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_json::parse_registry_history;
    use crate::schema_json::parse_schema_descriptor;
    use serde_json::json;

    const SCHEMA_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HISTORY_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";

    fn descriptor() -> SchemaDescriptor {
        let bytes = serde_json::to_vec(&json!({
            "id":SCHEMA_ID, "type":"core.schema", "schema":1,
            "fields":{
                "schema.targetType":"game.marker", "schema.targetVersion":1,
                "schema.fields":[
                    {"id":"marker.caption", "kind":"text", "presence":"optional", "maxUtf8Bytes":1024},
                    {"id":"marker.scene", "kind":"record-ref", "presence":"required", "targetType":"core.scene", "resolution":"required"}
                ]
            }
        })).unwrap();
        parse_schema_descriptor(&bytes).unwrap()
    }

    fn hex(bytes: &[u8; 32]) -> String {
        let mut text = String::with_capacity(64);
        for byte in bytes {
            use std::fmt::Write;
            write!(&mut text, "{byte:02x}").unwrap();
        }
        text
    }

    fn history_bytes(descriptor: &SchemaDescriptor) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "id":HISTORY_ID, "type":"core.registry-history", "schema":1,
            "fields":{
                "registry.fields":[
                    {"typeId":"game.marker", "fieldId":"marker.caption", "signature":hex(descriptor.fields()[0].signature()), "retired":false},
                    {"typeId":"game.marker", "fieldId":"marker.scene", "signature":hex(descriptor.fields()[1].signature()), "retired":false}
                ],
                "registry.versions":[
                    {"typeId":"game.marker", "targetVersion":1, "definitionHash":hex(descriptor.definition_hash())}
                ]
            }
        })).unwrap()
    }

    #[test]
    fn active_fields_and_version_join_without_allocating_a_second_history() {
        let descriptor = descriptor();
        let bytes = history_bytes(&descriptor);
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            validate_active_registry(std::slice::from_ref(&descriptor), &history),
            Ok(())
        );
        assert_eq!(
            validate_active_registry(&[descriptor.clone(), descriptor], &history),
            Err(RegistryConsistencyError::DescriptorOrder(1))
        );
    }

    #[test]
    fn false_active_field_or_version_refuses() {
        let descriptor = descriptor();
        let valid = history_bytes(&descriptor);
        let mut value: serde_json::Value = serde_json::from_slice(&valid).unwrap();

        value["fields"]["registry.fields"][0]["retired"] = json!(true);
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            validate_active_registry(std::slice::from_ref(&descriptor), &history),
            Err(RegistryConsistencyError::RetiredActiveField(0))
        );

        value["fields"]["registry.fields"][0]["retired"] = json!(false);
        value["fields"]["registry.fields"][0]["signature"] = json!("00".repeat(32));
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            validate_active_registry(std::slice::from_ref(&descriptor), &history),
            Err(RegistryConsistencyError::ChangedFieldMeaning(0))
        );

        value["fields"]["registry.fields"] = json!([]);
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            validate_active_registry(std::slice::from_ref(&descriptor), &history),
            Err(RegistryConsistencyError::MissingField(0, 0))
        );

        value = serde_json::from_slice(&valid).unwrap();
        value["fields"]["registry.versions"][0]["definitionHash"] = json!("00".repeat(32));
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            validate_active_registry(std::slice::from_ref(&descriptor), &history),
            Err(RegistryConsistencyError::ChangedVersionDefinition(0))
        );

        value["fields"]["registry.versions"] = json!([]);
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            validate_active_registry(&[descriptor], &history),
            Err(RegistryConsistencyError::MissingVersion(0))
        );
    }

    #[test]
    fn orphan_active_history_refuses_but_retired_history_remains() {
        let descriptor = descriptor();
        let valid = history_bytes(&descriptor);
        let mut value: serde_json::Value = serde_json::from_slice(&valid).unwrap();
        value["fields"]["registry.fields"].as_array_mut().unwrap().push(json!({
            "typeId":"game.marker", "fieldId":"marker.z-old", "signature":"00".repeat(32), "retired":false
        }));
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            validate_active_registry(std::slice::from_ref(&descriptor), &history),
            Err(RegistryConsistencyError::OrphanActiveField(2))
        );

        value["fields"]["registry.fields"][2]["retired"] = json!(true);
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(validate_active_registry(&[descriptor], &history), Ok(()));
    }

    #[test]
    fn multi_type_join_skips_retired_fields_and_older_versions() {
        let marker = descriptor();
        let other_bytes = serde_json::to_vec(&json!({
            "id":"4a5ef644-7225-4770-9f4d-e1d9c0a7d515", "type":"core.schema", "schema":1,
            "fields":{
                "schema.targetType":"game.other", "schema.targetVersion":2,
                "schema.fields":[
                    {"id":"other.caption", "kind":"text", "presence":"required", "maxUtf8Bytes":64}
                ]
            }
        }))
        .unwrap();
        let other = parse_schema_descriptor(&other_bytes).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&history_bytes(&marker)).unwrap();
        let fields = value["fields"]["registry.fields"].as_array_mut().unwrap();
        fields.insert(0, json!({
            "typeId":"game.aaa", "fieldId":"aaa.old", "signature":"00".repeat(32), "retired":true
        }));
        fields.insert(2, json!({
            "typeId":"game.marker", "fieldId":"marker.old", "signature":"00".repeat(32), "retired":true
        }));
        fields.push(json!({
            "typeId":"game.other", "fieldId":"other.caption", "signature":hex(other.fields()[0].signature()), "retired":false
        }));
        fields.push(json!({
            "typeId":"game.zzz", "fieldId":"zzz.old", "signature":"00".repeat(32), "retired":true
        }));
        let versions = value["fields"]["registry.versions"].as_array_mut().unwrap();
        versions.push(json!({
            "typeId":"game.other", "targetVersion":1, "definitionHash":"00".repeat(32)
        }));
        versions.push(json!({
            "typeId":"game.other", "targetVersion":2, "definitionHash":hex(other.definition_hash())
        }));
        let bytes = serde_json::to_vec(&value).unwrap();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(validate_active_registry(&[marker, other], &history), Ok(()));
    }
}
