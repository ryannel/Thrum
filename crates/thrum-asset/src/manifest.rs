use crate::container::Container;
use crate::layout::{MANIFEST_MAGIC, ManifestMeta, ManifestRecord, manifest_tag};
use crate::{AssetError, ContentHash, ProductBytes, ProductKind, Profile};

/// A validated, immutable manifest: every logical asset's product for each cooked profile.
///
/// Entries are sorted by `(name key, profile)`, so a lookup is a binary search over mapped
/// records with no parse-time allocation.
#[derive(Debug)]
pub struct Manifest {
    bytes: ProductBytes,
    generation: u64,
}

/// One resolved binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestEntry<'a> {
    /// Logical asset name, such as `meshes/rock`.
    pub name: &'a str,
    /// Product family.
    pub kind: ProductKind,
    /// Profile the product was cooked for.
    pub profile: Profile,
    /// Content address of the product.
    pub product: ContentHash,
    /// Product length.
    pub byte_len: u64,
}

impl Manifest {
    /// Validate manifest bytes: container structure, sort order, codes and names.
    ///
    /// # Errors
    /// Refuses any structural fault; the caller keeps its previous manifest.
    pub fn parse(bytes: ProductBytes) -> Result<Self, AssetError> {
        let generation = {
            let container = Container::parse(&bytes, MANIFEST_MAGIC)?;
            let meta: &ManifestMeta = container.one(manifest_tag::META)?;
            let records: &[ManifestRecord] = container.slice(manifest_tag::ENTRIES)?;
            let strings: &[u8] = container.slice(manifest_tag::STRINGS)?;
            if records.len() != meta.entry_count as usize {
                return Err(AssetError::Malformed("manifest entry count"));
            }
            for pair in records.windows(2) {
                if (pair[0].name_key, pair[0].profile) >= (pair[1].name_key, pair[1].profile) {
                    // Equal keys with different names would be a 64-bit collision; the cooker
                    // refuses to publish one, so any non-increasing pair is malformed.
                    return Err(AssetError::Malformed("manifest order"));
                }
            }
            for record in records {
                decode(record, strings)?;
            }
            meta.generation
        };
        Ok(Self { bytes, generation })
    }

    /// Publication generation recorded by the cooker.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Every entry in key order.
    pub fn entries(&self) -> impl Iterator<Item = ManifestEntry<'_>> {
        let (records, strings) = self.sections();
        records
            .iter()
            .filter_map(move |record| decode(record, strings).ok())
    }

    /// The product for `name` in the first profile of `preference` that has one.
    pub fn resolve(&self, name: &str, preference: &[Profile]) -> Option<ManifestEntry<'_>> {
        self.resolve_available(name, preference, |_| true)
    }

    pub(crate) fn resolve_available(
        &self,
        name: &str,
        preference: &[Profile],
        available: impl Fn(&ContentHash) -> bool,
    ) -> Option<ManifestEntry<'_>> {
        let key = ContentHash::of(name.as_bytes()).prefix_u64();
        let (records, strings) = self.sections();
        let start = records.partition_point(|record| record.name_key < key);
        let candidates = records[start..]
            .iter()
            .take_while(|record| record.name_key == key);
        preference.iter().find_map(|wanted| {
            candidates
                .clone()
                .filter_map(|record| decode(record, strings).ok())
                .find(|entry| {
                    entry.profile == *wanted && entry.name == name && available(&entry.product)
                })
        })
    }

    fn sections(&self) -> (&[ManifestRecord], &[u8]) {
        // Structure was validated in `parse` over these same immutable bytes.
        match Container::parse(&self.bytes, MANIFEST_MAGIC) {
            Ok(container) => (
                container.slice(manifest_tag::ENTRIES).unwrap_or_default(),
                container.slice(manifest_tag::STRINGS).unwrap_or_default(),
            ),
            Err(_) => (&[], &[]),
        }
    }
}

fn decode<'a>(record: &ManifestRecord, strings: &'a [u8]) -> Result<ManifestEntry<'a>, AssetError> {
    let start = record.name_offset as usize;
    let name = strings
        .get(start..start + usize::from(record.name_len))
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .ok_or(AssetError::Malformed("manifest name"))?;
    if ContentHash::of(name.as_bytes()).prefix_u64() != record.name_key {
        return Err(AssetError::Malformed("manifest name key"));
    }
    Ok(ManifestEntry {
        name,
        kind: ProductKind::from_code(record.kind).ok_or(AssetError::Malformed("manifest kind"))?,
        profile: Profile::from_code(record.profile)
            .ok_or(AssetError::Malformed("manifest profile"))?,
        product: ContentHash::from_bytes(record.product),
        byte_len: record.byte_len,
    })
}
