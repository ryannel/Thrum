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

impl MeaningEntry<'_> {
    fn key(&self) -> (&str, &str) {
        (self.type_id, self.field_id)
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

/// Compare a candidate history with one authority's pinned admitted history.
///
/// Both inputs must be strictly sorted and duplicate-free. Every prior key and
/// signature must remain, and retirement is one-way. New candidate keys are
/// permitted. This pure check allocates nothing and runs in O(prior + candidate)
/// time; it does not authenticate the prior root or calculate signatures.
pub fn compare_meaning_histories(
    prior: &[MeaningEntry<'_>],
    candidate: &[MeaningEntry<'_>],
) -> Result<(), MeaningHistoryError> {
    validate_order(prior, MeaningHistoryError::PriorOrder)?;
    validate_order(candidate, MeaningHistoryError::CandidateOrder)?;

    let mut candidate_index = 0;
    for (prior_index, old) in prior.iter().enumerate() {
        while let Some(new) = candidate.get(candidate_index) {
            match new.key().cmp(&old.key()) {
                Ordering::Less => candidate_index += 1,
                Ordering::Equal => break,
                Ordering::Greater => return Err(MeaningHistoryError::MissingPrior(prior_index)),
            }
        }
        let Some(new) = candidate.get(candidate_index) else {
            return Err(MeaningHistoryError::MissingPrior(prior_index));
        };
        if new.signature != old.signature {
            return Err(MeaningHistoryError::ChangedMeaning(prior_index));
        }
        if old.retired && !new.retired {
            return Err(MeaningHistoryError::RevivedRetired(prior_index));
        }
        candidate_index += 1;
    }
    Ok(())
}

fn validate_order(
    entries: &[MeaningEntry<'_>],
    error: fn(usize) -> MeaningHistoryError,
) -> Result<(), MeaningHistoryError> {
    for (index, pair) in entries.windows(2).enumerate() {
        if pair[0].key() >= pair[1].key() {
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
    #[ignore = "local linear 100,000-field lineage cost probe"]
    fn measure_meaning_history_100k() {
        let mut names = Vec::with_capacity(100_000);
        for index in 0..100_000_u32 {
            names.push(format!("field.{index:06}"));
        }
        let prior: Vec<_> = names
            .iter()
            .map(|field_id| entry("game.large", field_id, 7, false))
            .collect();
        let mut candidate = prior.clone();
        candidate.push(entry("game.large", "field.z", 8, false));
        let start = std::time::Instant::now();
        assert_eq!(compare_meaning_histories(&prior, &candidate), Ok(()));
        eprintln!(
            "lineage_entries=100000 compare_elapsed={:?}",
            start.elapsed()
        );
    }
}
