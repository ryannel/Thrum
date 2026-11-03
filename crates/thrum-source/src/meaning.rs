//! Pure monotonic comparison of one F3 field-meaning lineage.
//!
//! A future source descriptor validator must derive each signature from fixed
//! mechanical attributes. The durable admission owner must supply the prior
//! history from its current admitted root under the root-generation CAS.

use std::cmp::Ordering;

/// One `(type ID, field ID)` meaning retained by a source-carried registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeaningEntry<'a> {
    /// Stable registered type ID.
    pub type_id: &'a str,
    /// Permanent field ID within that type.
    pub field_id: &'a str,
    /// Digest of canonical interpretation-bearing descriptor attributes.
    pub signature: [u8; 32],
    /// A retired ID stays reserved and cannot become active again.
    pub retired: bool,
}

pub(crate) trait FieldHistoryView {
    fn history_key(&self) -> (&str, &str);
    fn history_signature(&self) -> &[u8; 32];
    fn is_retired(&self) -> bool;
}

impl FieldHistoryView for MeaningEntry<'_> {
    fn history_key(&self) -> (&str, &str) {
        (self.type_id, self.field_id)
    }

    fn history_signature(&self) -> &[u8; 32] {
        &self.signature
    }

    fn is_retired(&self) -> bool {
        self.retired
    }
}

/// A candidate registry cannot extend its pinned predecessor monotonically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeaningHistoryError {
    /// Prior entries are not strictly sorted by `(type ID, field ID)`.
    PriorOrder(usize),
    /// Candidate entries are not strictly sorted by `(type ID, field ID)`.
    CandidateOrder(usize),
    /// A previously admitted ID vanished from candidate history.
    MissingPrior(usize),
    /// A previously admitted ID has a different mechanical signature.
    ChangedMeaning(usize),
    /// A retired ID became active again.
    RevivedRetired(usize),
}

/// One immutable definition retained for a `(type ID, target schema version)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeVersionEntry<'a> {
    /// Stable registered type ID.
    pub type_id: &'a str,
    /// Positive target schema version, checked by the descriptor parser.
    pub version: u64,
    /// Digest of the canonical field set and optional-extension policy.
    pub definition_hash: [u8; 32],
}

pub(crate) trait TypeVersionHistoryView {
    fn history_key(&self) -> (&str, u64);
    fn history_definition_hash(&self) -> &[u8; 32];
}

impl TypeVersionHistoryView for TypeVersionEntry<'_> {
    fn history_key(&self) -> (&str, u64) {
        (self.type_id, self.version)
    }

    fn history_definition_hash(&self) -> &[u8; 32] {
        &self.definition_hash
    }
}

/// A candidate type-version history cannot extend its pinned predecessor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeVersionHistoryError {
    /// Prior entries are not strictly sorted by `(type ID, version)`.
    PriorOrder(usize),
    /// Candidate entries are not strictly sorted by `(type ID, version)`.
    CandidateOrder(usize),
    /// A previously admitted type version vanished from candidate history.
    MissingPrior(usize),
    /// A previously admitted type version now has a different definition.
    ChangedDefinition(usize),
}

/// Compare a candidate history with one authority's pinned admitted history.
///
/// Both inputs must be strictly sorted and duplicate-free. Every prior key and
/// signature must remain, and retirement is one-way. New candidate keys are
/// permitted. This pure check allocates nothing and runs in O(prior + candidate)
/// time; it does not authenticate the prior root or calculate signatures.
///
/// # Errors
///
/// A candidate field meaning is missing, changed, or improperly retired.
pub fn compare_meaning_histories(
    prior: &[MeaningEntry<'_>],
    candidate: &[MeaningEntry<'_>],
) -> Result<(), MeaningHistoryError> {
    compare_field_history_views(prior, candidate)
}

pub(crate) fn compare_field_history_views<P: FieldHistoryView, C: FieldHistoryView>(
    prior: &[P],
    candidate: &[C],
) -> Result<(), MeaningHistoryError> {
    validate_field_order(prior, MeaningHistoryError::PriorOrder)?;
    validate_field_order(candidate, MeaningHistoryError::CandidateOrder)?;

    let mut candidate_index = 0;
    for (prior_index, old) in prior.iter().enumerate() {
        while let Some(new) = candidate.get(candidate_index) {
            match new.history_key().cmp(&old.history_key()) {
                Ordering::Less => candidate_index += 1,
                Ordering::Equal => break,
                Ordering::Greater => return Err(MeaningHistoryError::MissingPrior(prior_index)),
            }
        }
        let Some(new) = candidate.get(candidate_index) else {
            return Err(MeaningHistoryError::MissingPrior(prior_index));
        };
        if new.history_signature() != old.history_signature() {
            return Err(MeaningHistoryError::ChangedMeaning(prior_index));
        }
        if old.is_retired() && !new.is_retired() {
            return Err(MeaningHistoryError::RevivedRetired(prior_index));
        }
        candidate_index += 1;
    }
    Ok(())
}

/// Require an immutable definition for every previously admitted type version.
///
/// Both slices must be strictly sorted and duplicate-free. New version keys are
/// allowed. This pure, allocation-free O(prior + candidate) check does not
/// derive definition hashes or authenticate the prior admitted root.
///
/// # Errors
///
/// A candidate type-version definition is missing, changed, or improperly retired.
pub fn compare_type_version_histories(
    prior: &[TypeVersionEntry<'_>],
    candidate: &[TypeVersionEntry<'_>],
) -> Result<(), TypeVersionHistoryError> {
    compare_type_version_views(prior, candidate)
}

pub(crate) fn compare_type_version_views<P: TypeVersionHistoryView, C: TypeVersionHistoryView>(
    prior: &[P],
    candidate: &[C],
) -> Result<(), TypeVersionHistoryError> {
    for (index, pair) in prior.windows(2).enumerate() {
        if pair[0].history_key() >= pair[1].history_key() {
            return Err(TypeVersionHistoryError::PriorOrder(index + 1));
        }
    }
    for (index, pair) in candidate.windows(2).enumerate() {
        if pair[0].history_key() >= pair[1].history_key() {
            return Err(TypeVersionHistoryError::CandidateOrder(index + 1));
        }
    }

    let mut candidate_index = 0;
    for (prior_index, old) in prior.iter().enumerate() {
        while let Some(new) = candidate.get(candidate_index) {
            match new.history_key().cmp(&old.history_key()) {
                Ordering::Less => candidate_index += 1,
                Ordering::Equal => break,
                Ordering::Greater => {
                    return Err(TypeVersionHistoryError::MissingPrior(prior_index));
                }
            }
        }
        let Some(new) = candidate.get(candidate_index) else {
            return Err(TypeVersionHistoryError::MissingPrior(prior_index));
        };
        if new.history_definition_hash() != old.history_definition_hash() {
            return Err(TypeVersionHistoryError::ChangedDefinition(prior_index));
        }
        candidate_index += 1;
    }
    Ok(())
}

fn validate_field_order<T: FieldHistoryView>(
    entries: &[T],
    error: fn(usize) -> MeaningHistoryError,
) -> Result<(), MeaningHistoryError> {
    for (index, pair) in entries.windows(2).enumerate() {
        if pair[0].history_key() >= pair[1].history_key() {
            return Err(error(index + 1));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn entry<'a>(
        type_id: &'a str,
        field_id: &'a str,
        signature: u8,
        retired: bool,
    ) -> MeaningEntry<'a> {
        MeaningEntry {
            type_id,
            field_id,
            signature: [signature; 32],
            retired,
        }
    }

    #[test]
    fn monotonic_union_and_one_way_retirement() {
        let prior = [
            entry("game.marker", "marker.caption", 1, false),
            entry("game.marker", "marker.target", 2, false),
            entry("game.old", "old.field", 3, true),
        ];
        let candidate = [
            entry("game.marker", "marker.caption", 1, false),
            entry("game.marker", "marker.target", 2, true),
            entry("game.marker", "marker.tint", 4, false),
            entry("game.old", "old.field", 3, true),
        ];
        assert_eq!(compare_meaning_histories(&prior, &candidate), Ok(()));
        assert_eq!(compare_meaning_histories(&[], &candidate), Ok(()));

        let missing = [candidate[0], candidate[2], candidate[3]];
        assert_eq!(
            compare_meaning_histories(&prior, &missing),
            Err(MeaningHistoryError::MissingPrior(1))
        );
        let changed = [
            candidate[0],
            entry("game.marker", "marker.target", 9, false),
            prior[2],
        ];
        assert_eq!(
            compare_meaning_histories(&prior, &changed),
            Err(MeaningHistoryError::ChangedMeaning(1))
        );
        let revived = [prior[0], prior[1], entry("game.old", "old.field", 3, false)];
        assert_eq!(
            compare_meaning_histories(&prior, &revived),
            Err(MeaningHistoryError::RevivedRetired(2))
        );
    }

    #[test]
    fn branch_history_union_requires_prior_ids_and_strict_order() {
        let prior = [
            entry("game.marker", "marker.caption", 1, false),
            entry("game.marker", "marker.target", 2, false),
        ];
        let stale_branch = [prior[0], entry("game.marker", "marker.tint", 4, false)];
        assert_eq!(
            compare_meaning_histories(&prior, &stale_branch),
            Err(MeaningHistoryError::MissingPrior(1))
        );
        let merged_branch = [prior[0], prior[1], stale_branch[1]];
        assert_eq!(compare_meaning_histories(&prior, &merged_branch), Ok(()));

        let duplicate = [prior[0], prior[0]];
        assert_eq!(
            compare_meaning_histories(&prior, &duplicate),
            Err(MeaningHistoryError::CandidateOrder(1))
        );
        let backwards = [prior[1], prior[0]];
        assert_eq!(
            compare_meaning_histories(&backwards, &merged_branch),
            Err(MeaningHistoryError::PriorOrder(1))
        );
    }

    #[test]
    fn type_versions_are_immutable_and_branch_union_is_explicit() {
        let v1 = TypeVersionEntry {
            type_id: "game.marker",
            version: 1,
            definition_hash: [1; 32],
        };
        let v2 = TypeVersionEntry {
            type_id: "game.marker",
            version: 2,
            definition_hash: [2; 32],
        };
        let other = TypeVersionEntry {
            type_id: "game.other",
            version: 1,
            definition_hash: [3; 32],
        };
        assert_eq!(
            compare_type_version_histories(&[v1], &[v1, v2, other]),
            Ok(())
        );
        assert_eq!(
            compare_type_version_histories(&[v1, v2], &[v1, other]),
            Err(TypeVersionHistoryError::MissingPrior(1))
        );
        let redefined = TypeVersionEntry {
            definition_hash: [9; 32],
            ..v1
        };
        assert_eq!(
            compare_type_version_histories(&[v1], &[redefined]),
            Err(TypeVersionHistoryError::ChangedDefinition(0))
        );
        assert_eq!(
            compare_type_version_histories(&[v1], &[v1, v1]),
            Err(TypeVersionHistoryError::CandidateOrder(1))
        );
        assert_eq!(
            compare_type_version_histories(&[v2, v1], &[v1, v2]),
            Err(TypeVersionHistoryError::PriorOrder(1))
        );
        assert_eq!(
            compare_type_version_histories(&[v1, v2], &[v1, v2, other]),
            Ok(())
        );
    }
}
