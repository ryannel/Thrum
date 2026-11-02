//! First F4 headless scene product from an accepted, pinned source root.
//!
//! This product is not a runnable build: no game-code or toolchain closure is
//! bound yet. Cooking never opens the mutable working tree.

use sha2::{Digest, Sha256};
use std::io;
mod build_declaration;
mod native_worktree;
pub use build_declaration::{
    BuildDeclaration, BuildDeclarationError, read_pinned_build_declaration,
};
pub use native_worktree::{
    BuildScratchPlan, MaterializeError, MaterializedSource, materialize_pinned_source,
};
use thrum_source::{
    AcceptedSourceRoot, MAX_RECORD_BYTES, PinnedSource, SourcePath, SourceStoreError, parse_scene,
    project_v2_json::parse_project_v2,
};

const COOK_FORMAT: &[u8] = b"thrum.scene-product.v1\0";

/// The sole selected target for this first content product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneTarget {
    /// CPU-only entry scene; gameplay code and world activation follow later.
    Headless,
}

/// Digest of one target-specific derived scene product, distinct from source
/// and from a future complete `BuildRevision`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneProductRevision([u8; 32]);

impl SceneProductRevision {
    /// Raw digest for a future product manifest or diagnostic.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Immutable first-profile scene content and its admitted provenance.
pub struct SceneProduct {
    root: AcceptedSourceRoot,
    target: SceneTarget,
    revision: SceneProductRevision,
    project_input_hash: [u8; 32],
    scene_input_hash: [u8; 32],
    scene_id: String,
    label: Option<String>,
}

impl SceneProduct {
    /// Exact admitted root used to derive this product.
    pub fn source_root(&self) -> AcceptedSourceRoot {
        self.root
    }

    /// Product target; this is not a runnable-build target declaration.
    pub fn target(&self) -> SceneTarget {
        self.target
    }

    /// Domain-separated deterministic product identity.
    pub fn revision(&self) -> SceneProductRevision {
        self.revision
    }

    /// Stable authored entry-scene ID.
    pub fn scene_id(&self) -> &str {
        &self.scene_id
    }

    /// Optional authored caption.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Encode this immutable product for a future content-addressed manifest.
    /// The independent byte digest must be retained by that manifest and
    /// supplied to `load_scene_artifact` before a consumer uses these bytes.
    pub fn artifact(&self) -> SceneArtifact {
        let label_bytes = self
            .label
            .as_ref()
            .map_or(&[][..], |label| label.as_bytes());
        let label_len = u32::try_from(label_bytes.len()).expect("bounded source record");
        let mut bytes = Vec::with_capacity(COOK_FORMAT.len() + 138 + label_bytes.len());
        bytes.extend_from_slice(COOK_FORMAT);
        bytes.push(target_byte(self.target));
        bytes.extend_from_slice(self.revision.as_bytes());
        bytes.extend_from_slice(&self.project_input_hash);
        bytes.extend_from_slice(&self.scene_input_hash);
        bytes.extend_from_slice(self.scene_id.as_bytes());
        bytes.push(u8::from(self.label.is_some()));
        bytes.extend_from_slice(&label_len.to_le_bytes());
        bytes.extend_from_slice(label_bytes);
        SceneArtifact {
            content_hash: Sha256::digest(&bytes).into(),
            bytes,
        }
    }
}

/// Serialized product bytes plus a digest for a future trusted build manifest.
pub struct SceneArtifact {
    bytes: Vec<u8>,
    content_hash: [u8; 32],
}

impl SceneArtifact {
    /// Exact encoded product bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// SHA-256 of `bytes`; the build manifest must retain this separately.
    pub fn content_hash(&self) -> [u8; 32] {
        self.content_hash
    }
}

/// A serialized product cannot satisfy a trusted manifest entry.
#[derive(Debug, Eq, PartialEq)]
pub enum SceneLoadError {
    /// Encoded bytes differ from the manifest's expected content hash.
    ContentHash,
    /// Truncated, oversized or unsupported product encoding.
    Format,
    /// Encoded product key disagrees with its inputs or the manifest.
    Revision,
}

/// Load verified product bytes using expected identities from a trusted build
/// manifest. Root provenance comes from that manifest, not the reusable
/// product bytes. The caller must verify the manifest and its source/code closure.
pub fn load_scene_artifact(
    bytes: &[u8],
    expected_content_hash: [u8; 32],
    expected_root: AcceptedSourceRoot,
    expected_revision: SceneProductRevision,
) -> Result<SceneProduct, SceneLoadError> {
    if bytes.len() > COOK_FORMAT.len() + 138 + MAX_RECORD_BYTES {
        return Err(SceneLoadError::Format);
    }
    if Sha256::digest(bytes).as_slice() != expected_content_hash {
        return Err(SceneLoadError::ContentHash);
    }
    let mut cursor = bytes;
    if take(&mut cursor, COOK_FORMAT.len())? != COOK_FORMAT {
        return Err(SceneLoadError::Format);
    }
    let target = match take(&mut cursor, 1)?[0] {
        1 => SceneTarget::Headless,
        _ => return Err(SceneLoadError::Format),
    };
    let revision = take(&mut cursor, 32)?;
    let project_input_hash: [u8; 32] = take(&mut cursor, 32)?.try_into().expect("fixed slice");
    let scene_input_hash: [u8; 32] = take(&mut cursor, 32)?.try_into().expect("fixed slice");
    let derived = derive_revision(target, project_input_hash, scene_input_hash);
    if revision != expected_revision.as_bytes() || derived != expected_revision {
        return Err(SceneLoadError::Revision);
    }
    let id = std::str::from_utf8(take(&mut cursor, 36)?)
        .map_err(|_| SceneLoadError::Format)?
        .to_owned();
    if !canonical_uuid(&id) {
        return Err(SceneLoadError::Format);
    }
    let has_label = take(&mut cursor, 1)?[0];
    if has_label > 1 {
        return Err(SceneLoadError::Format);
    }
    let label_len = u32::from_le_bytes(take(&mut cursor, 4)?.try_into().expect("fixed slice"));
    if label_len as usize > MAX_RECORD_BYTES || (has_label == 0 && label_len != 0) {
        return Err(SceneLoadError::Format);
    }
    let label_bytes = take(&mut cursor, label_len as usize)?;
    if !cursor.is_empty() {
        return Err(SceneLoadError::Format);
    }
    let label = if has_label == 1 {
        Some(
            std::str::from_utf8(label_bytes)
                .map_err(|_| SceneLoadError::Format)?
                .to_owned(),
        )
    } else {
        None
    };
    Ok(SceneProduct {
        root: expected_root,
        target,
        revision: expected_revision,
        project_input_hash,
        scene_input_hash,
        scene_id: id,
        label,
    })
}

fn take<'a>(cursor: &mut &'a [u8], len: usize) -> Result<&'a [u8], SceneLoadError> {
    if len > cursor.len() {
        return Err(SceneLoadError::Format);
    }
    let (value, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(value)
}

fn canonical_uuid(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 36
        && bytes
            .iter()
            .copied()
            .enumerate()
            .all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
            })
        && matches!(bytes[14], b'1'..=b'8')
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

fn target_byte(target: SceneTarget) -> u8 {
    match target {
        SceneTarget::Headless => 1,
    }
}

fn derive_revision(
    target: SceneTarget,
    project_input_hash: [u8; 32],
    scene_input_hash: [u8; 32],
) -> SceneProductRevision {
    let mut digest = Sha256::new();
    digest.update(COOK_FORMAT);
    digest.update([target_byte(target)]);
    digest.update(project_input_hash);
    digest.update(scene_input_hash);
    SceneProductRevision(digest.finalize().into())
}

/// Verified match between one admitted rule source file and bytes embedded by a
/// statically linked game. This is a source-code match, not binary attestation.
pub struct LinkedRuleSource {
    path: SourcePath,
    sha256: [u8; 32],
}

impl LinkedRuleSource {
    /// Admitted path whose exact contents were checked.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// SHA-256 of the matched source bytes.
    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

/// A statically linked game's declared rule source is not the pinned source.
#[derive(Debug)]
pub enum LinkedRuleError {
    /// The declared code path is absent from this source revision.
    Missing,
    /// Embedded source bytes differ from the admitted file identity.
    Mismatch,
    /// Verified source read failed; the code match is not established.
    Source(SourceStoreError),
}

/// Verify a compiled game's declared source length/digest against one admitted
/// code file. The caller's build metadata needs separate T2 attestation; it
/// need not embed whole source files in a production binary. This bounds
/// memory to the existing pin scratch and checks stored bytes. A complete
/// ready build must also bind the actual binary, toolchain/dependency closure
/// and target manifest.
pub fn verify_linked_rule_source(
    pin: &mut PinnedSource<'_>,
    path: &SourcePath,
    linked_length: u64,
    linked_sha256: [u8; 32],
) -> Result<LinkedRuleSource, LinkedRuleError> {
    let entry = pin
        .manifest()
        .files()
        .binary_search_by(|file| file.path().cmp(path))
        .ok()
        .map(|index| &pin.manifest().files()[index])
        .ok_or(LinkedRuleError::Missing)?;
    let (length, admitted_hash) = entry.content_identity();
    if length != linked_length || admitted_hash != linked_sha256 {
        return Err(LinkedRuleError::Mismatch);
    }
    pin.copy_file_to(path, &mut io::sink())
        .map_err(LinkedRuleError::Source)?;
    Ok(LinkedRuleSource {
        path: path.clone(),
        sha256: admitted_hash,
    })
}

/// A pinned source cannot produce this bounded first-profile scene product.
#[derive(Debug)]
pub enum SceneCookError {
    /// Verified source read failed; output must be discarded.
    Source(SourceStoreError),
    /// Project bytes or identity disagree with the accepted manifest.
    Project,
    /// This cooker currently accepts exactly one entry scene.
    EntryCount,
    /// Entry ID has no declared scene record in the manifest.
    EntryMissing,
    /// Scene bytes or identity disagree with the accepted manifest/project.
    Scene,
    /// A record would exceed the fixed first-profile parser bound.
    RecordTooLarge,
}

/// Cook one headless entry scene using only an accepted read pin.
///
/// The pinned manifest locates the scene by stable record identity. Unselected
/// scene bytes are not read or decoded. The caller retains any prior ready
/// product if this returns an error; no active run is mutated here.
pub fn cook_entry_scene(
    pin: &mut PinnedSource<'_>,
    target: SceneTarget,
) -> Result<SceneProduct, SceneCookError> {
    let root = pin.root();
    let project_path = SourcePath::parse("project.json").expect("fixed portable path");
    let project_bytes = read_record(pin, &project_path)?;
    let project = parse_project_v2(&project_bytes).map_err(|_| SceneCookError::Project)?;
    if project.id() != pin.manifest().project_id() {
        return Err(SceneCookError::Project);
    }
    let [entry_id] = project.entry_scenes() else {
        return Err(SceneCookError::EntryCount);
    };
    let scene_path = pin
        .manifest()
        .files()
        .iter()
        .find(|file| file.record_identity() == Some((entry_id.as_str(), "core.scene")))
        .map(|file| file.path().clone())
        .ok_or(SceneCookError::EntryMissing)?;
    if project.record_paths().binary_search(&scene_path).is_err() {
        return Err(SceneCookError::EntryMissing);
    }
    let scene_bytes = read_record(pin, &scene_path)?;
    let scene = parse_scene(&scene_bytes).map_err(|_| SceneCookError::Scene)?;
    if scene.id != *entry_id {
        return Err(SceneCookError::Scene);
    }
    // Only declared direct inputs enter the cook key. An unrelated admitted
    // file may advance SourceRevision without invalidating this product.
    let project_input_hash = input_hash(pin, &project_path)?;
    let scene_input_hash = input_hash(pin, &scene_path)?;
    Ok(SceneProduct {
        root,
        target,
        revision: derive_revision(target, project_input_hash, scene_input_hash),
        project_input_hash,
        scene_input_hash,
        scene_id: scene.id,
        label: scene.label,
    })
}

fn input_hash(pin: &PinnedSource<'_>, path: &SourcePath) -> Result<[u8; 32], SceneCookError> {
    pin.manifest()
        .files()
        .binary_search_by(|file| file.path().cmp(path))
        .ok()
        .map(|index| pin.manifest().files()[index].content_identity().1)
        .ok_or(SceneCookError::EntryMissing)
}

fn read_record(pin: &mut PinnedSource<'_>, path: &SourcePath) -> Result<Vec<u8>, SceneCookError> {
    let entry = pin
        .manifest()
        .files()
        .binary_search_by(|file| file.path().cmp(path))
        .ok()
        .map(|index| &pin.manifest().files()[index])
        .ok_or(SceneCookError::EntryMissing)?;
    let (length, _) = entry.content_identity();
    let capacity = usize::try_from(length).map_err(|_| SceneCookError::RecordTooLarge)?;
    if capacity > MAX_RECORD_BYTES {
        return Err(SceneCookError::RecordTooLarge);
    }
    let mut bytes = Vec::with_capacity(capacity);
    pin.copy_file_to(path, &mut bytes)
        .map_err(SceneCookError::Source)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    use std::ffi::OsString;
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;
    use thrum_source::{
        NativeSourceStore, SourceSpacePlan, registry_stage::validate_staged_candidate,
        stage_declared_tree_with_plan,
    };

    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const OTHER_ID: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HISTORY_ID: &str = "6a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "thrum-cook-{}-{}",
                std::process::id(),
                sequence
            ));
            fs::create_dir(&path).unwrap();
            fs::create_dir(path.join("scenes")).unwrap();
            fs::create_dir(path.join("schemas")).unwrap();
            fs::create_dir(path.join(".engine")).unwrap();
            Self(path)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn plan() -> SourceSpacePlan {
            SourceSpacePlan {
                main_target_bytes: 1024 * 1024 * 1024,
                wal_stop_bytes: 64 * 1024 * 1024,
                temporary_target_bytes: 64 * 1024 * 1024,
                free_headroom_bytes: 0,
            }
        }

        fn build_plan() -> BuildScratchPlan {
            BuildScratchPlan {
                temporary_target_bytes: 64 * 1024 * 1024,
                free_headroom_bytes: 0,
            }
        }

        fn write(&self, caption: &str, second_entry: bool, other_file: bool) -> Vec<SourcePath> {
            let scenes = if second_entry {
                vec!["scenes/main.scene.json", "scenes/other.scene.json"]
            } else {
                vec!["scenes/main.scene.json"]
            };
            let entries = if second_entry {
                vec![SCENE_ID, OTHER_ID]
            } else {
                vec![SCENE_ID]
            };
            fs::write(
                self.root().join("project.json"),
                serde_json::to_vec(&serde_json::json!({
                    "id":PROJECT_ID,"type":"core.project","schema":2,
                    "fields":{"project.format":1,"project.records":scenes,
                    "project.schemas":[],"project.registryHistory":"schemas/history.registry.json",
                    "project.entryScenes":entries}
                }))
                .unwrap(),
            )
            .unwrap();
            fs::write(
                self.root().join("scenes/main.scene.json"),
                serde_json::to_vec(&serde_json::json!({
                    "id":SCENE_ID,"type":"core.scene","schema":1,
                    "fields":{"scene.label":caption}
                }))
                .unwrap(),
            )
            .unwrap();
            if second_entry {
                fs::write(
                    self.root().join("scenes/other.scene.json"),
                    serde_json::to_vec(&serde_json::json!({
                        "id":OTHER_ID,"type":"core.scene","schema":1,"fields":{}
                    }))
                    .unwrap(),
                )
                .unwrap();
            }
            fs::write(
                self.root().join("schemas/history.registry.json"),
                serde_json::to_vec(&serde_json::json!({
                    "id":HISTORY_ID,"type":"core.registry-history","schema":1,
                    "fields":{"registry.fields":[],"registry.versions":[]}
                }))
                .unwrap(),
            )
            .unwrap();
            let mut paths = vec![
                "project.json",
                "scenes/main.scene.json",
                "schemas/history.registry.json",
            ];
            if second_entry {
                paths.push("scenes/other.scene.json");
            }
            if other_file {
                fs::write(self.root().join("notes.txt"), "unrelated").unwrap();
                paths.push("notes.txt");
            }
            paths.sort_unstable();
            paths
                .into_iter()
                .map(|path| SourcePath::parse(path).unwrap())
                .collect()
        }

        fn admit(
            &self,
            store: &mut NativeSourceStore,
            generation: i64,
            key: i64,
            paths: &[SourcePath],
        ) -> AcceptedSourceRoot {
            let stage = stage_declared_tree_with_plan(self.root(), paths, Self::plan()).unwrap();
            let candidate = validate_staged_candidate(&stage, &[]).unwrap();
            store
                .admit(generation, 1, key, [key as u8; 32], &candidate)
                .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn admitted_pins_cook_exact_scene_and_failed_replacement_keeps_prior_product() {
        let fixture = Fixture::new();
        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let a_paths = fixture.write("A", false, false);
        let a = fixture.admit(&mut store, 0, 1, &a_paths);
        let mut a_pin = store.pin_current(a.generation).unwrap();
        fs::write(
            fixture.root().join("scenes/main.scene.json"),
            b"broken checkout",
        )
        .unwrap();
        let prior = cook_entry_scene(&mut a_pin, SceneTarget::Headless).unwrap();
        assert_eq!(prior.label(), Some("A"));
        assert_eq!(prior.source_root(), a);
        let artifact = prior.artifact();
        let loaded = load_scene_artifact(
            artifact.bytes(),
            artifact.content_hash(),
            a,
            prior.revision(),
        )
        .unwrap();
        assert_eq!(loaded.label(), Some("A"));
        assert_eq!(loaded.revision(), prior.revision());
        let mut damaged = artifact.bytes().to_vec();
        *damaged.last_mut().unwrap() ^= 1;
        assert_eq!(
            load_scene_artifact(&damaged, artifact.content_hash(), a, prior.revision())
                .err()
                .unwrap(),
            SceneLoadError::ContentHash
        );
        let truncated = &artifact.bytes()[..artifact.bytes().len() - 1];
        assert_eq!(
            load_scene_artifact(
                truncated,
                Sha256::digest(truncated).into(),
                a,
                prior.revision()
            )
            .err()
            .unwrap(),
            SceneLoadError::Format
        );
        a_pin.close().unwrap();

        let b_paths = fixture.write("B", false, false);
        let b = fixture.admit(&mut store, a.generation, 2, &b_paths);
        let mut b_pin = store.pin_current(b.generation).unwrap();
        let b_product = cook_entry_scene(&mut b_pin, SceneTarget::Headless).unwrap();
        assert_eq!(b_product.label(), Some("B"));
        assert_ne!(b_product.revision(), prior.revision());
        assert_eq!(
            load_scene_artifact(
                artifact.bytes(),
                artifact.content_hash(),
                a,
                b_product.revision()
            )
            .err()
            .unwrap(),
            SceneLoadError::Revision
        );
        b_pin.close().unwrap();

        let c_paths = fixture.write("B", false, true);
        let c = fixture.admit(&mut store, b.generation, 3, &c_paths);
        let mut c_pin = store.pin_current(c.generation).unwrap();
        let c_product = cook_entry_scene(&mut c_pin, SceneTarget::Headless).unwrap();
        assert_ne!(c_product.source_root(), b_product.source_root());
        assert_eq!(c_product.revision(), b_product.revision());
        assert_eq!(c_product.artifact().bytes(), b_product.artifact().bytes());
        assert_eq!(
            c_product.artifact().content_hash(),
            b_product.artifact().content_hash()
        );
        c_pin.close().unwrap();

        let d_paths = fixture.write("D", true, true);
        let d = fixture.admit(&mut store, c.generation, 4, &d_paths);
        let mut d_pin = store.pin_current(d.generation).unwrap();
        assert!(matches!(
            cook_entry_scene(&mut d_pin, SceneTarget::Headless),
            Err(SceneCookError::EntryCount)
        ));
        assert_eq!(prior.label(), Some("A"));
        assert_eq!(prior.source_root(), a);
        d_pin.close().unwrap();
        let mut historical = store.pin_accepted(a).unwrap();
        let historical_product = cook_entry_scene(&mut historical, SceneTarget::Headless).unwrap();
        assert_eq!(historical_product.revision(), prior.revision());
        historical.close().unwrap();
    }

    #[test]
    fn absent_and_empty_captions_remain_distinct_after_checked_load() {
        let fixture = Fixture::new();
        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let paths = fixture.write("", false, false);
        let empty_root = fixture.admit(&mut store, 0, 1, &paths);
        let mut empty_pin = store.pin_current(empty_root.generation).unwrap();
        let empty = cook_entry_scene(&mut empty_pin, SceneTarget::Headless).unwrap();
        assert_eq!(empty.label(), Some(""));
        let empty_artifact = empty.artifact();
        let empty_loaded = load_scene_artifact(
            empty_artifact.bytes(),
            empty_artifact.content_hash(),
            empty_root,
            empty.revision(),
        )
        .unwrap();
        assert_eq!(empty_loaded.label(), Some(""));
        empty_pin.close().unwrap();

        fs::write(
            fixture.root().join("scenes/main.scene.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":SCENE_ID,"type":"core.scene","schema":1,"fields":{}
            }))
            .unwrap(),
        )
        .unwrap();
        let absent_root = fixture.admit(&mut store, empty_root.generation, 2, &paths);
        let mut absent_pin = store.pin_current(absent_root.generation).unwrap();
        let absent = cook_entry_scene(&mut absent_pin, SceneTarget::Headless).unwrap();
        assert_eq!(absent.label(), None);
        assert_ne!(absent.revision(), empty.revision());
        let absent_artifact = absent.artifact();
        let absent_loaded = load_scene_artifact(
            absent_artifact.bytes(),
            absent_artifact.content_hash(),
            absent_root,
            absent.revision(),
        )
        .unwrap();
        assert_eq!(absent_loaded.label(), None);
        absent_pin.close().unwrap();
    }

    #[test]
    fn admitted_rule_bytes_match_linked_bytes_and_a_new_code_edit_refuses() {
        const LINKED: &[u8] = b"pub fn decide() -> u64 { 1 }\n";
        let fixture = Fixture::new();
        fs::create_dir(fixture.root().join("game")).unwrap();
        let rules_path = SourcePath::parse("game/rules.rs").unwrap();
        let mut paths = fixture.write("rules", false, false);
        paths.push(rules_path.clone());
        paths.sort_unstable();
        fs::write(fixture.root().join(rules_path.as_str()), LINKED).unwrap();
        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let a = fixture.admit(&mut store, 0, 1, &paths);
        fs::write(
            fixture.root().join(rules_path.as_str()),
            vec![b'X'; LINKED.len()],
        )
        .unwrap();
        let mut a_pin = store.pin_current(a.generation).unwrap();
        let linked_hash: [u8; 32] = Sha256::digest(LINKED).into();
        let matched =
            verify_linked_rule_source(&mut a_pin, &rules_path, LINKED.len() as u64, linked_hash)
                .unwrap();
        assert_eq!(matched.path(), &rules_path);
        assert_eq!(matched.sha256(), linked_hash);
        a_pin.close().unwrap();
        let b = fixture.admit(&mut store, a.generation, 2, &paths);
        let mut b_pin = store.pin_current(b.generation).unwrap();
        assert!(matches!(
            verify_linked_rule_source(&mut b_pin, &rules_path, LINKED.len() as u64, linked_hash),
            Err(LinkedRuleError::Mismatch)
        ));
        b_pin.close().unwrap();
        let mut historical = store.pin_accepted(a).unwrap();
        verify_linked_rule_source(
            &mut historical,
            &rules_path,
            LINKED.len() as u64,
            linked_hash,
        )
        .unwrap();
        historical.close().unwrap();
    }

    #[test]
    fn materialized_build_inputs_are_exact_pinned_bytes_and_owned_temporary_files() {
        let fixture = Fixture::new();
        let paths = fixture.write("admitted caption", false, false);
        let original_scene = fs::read(fixture.root().join("scenes/main.scene.json")).unwrap();
        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let root = fixture.admit(&mut store, 0, 1, &paths);
        fs::write(
            fixture.root().join("scenes/main.scene.json"),
            b"dirty checkout",
        )
        .unwrap();
        let mut pin = store.pin_current(root.generation).unwrap();
        let destination = fixture.root().join(".engine/build-inputs");
        let materialized =
            materialize_pinned_source(&mut pin, &destination, Fixture::build_plan()).unwrap();
        assert_eq!(materialized.source_root(), root);
        assert_eq!(materialized.file_count(), paths.len());
        assert_eq!(
            materialized.copied_bytes(),
            pin.manifest()
                .files()
                .iter()
                .map(|file| file.content_identity().0)
                .sum::<u64>()
        );
        assert_eq!(
            fs::read(materialized.path().join("scenes/main.scene.json")).unwrap(),
            original_scene
        );
        assert!(!materialized.path().join(".engine").exists());
        materialized.close().unwrap();
        assert!(!destination.exists());

        let dropped =
            materialize_pinned_source(&mut pin, &destination, Fixture::build_plan()).unwrap();
        drop(dropped);
        assert!(!destination.exists());

        assert!(matches!(
            materialize_pinned_source(
                &mut pin,
                std::path::Path::new("relative-build-inputs"),
                Fixture::build_plan()
            ),
            Err(MaterializeError::InvalidDestination)
        ));

        assert!(matches!(
            materialize_pinned_source(
                &mut pin,
                &destination,
                BuildScratchPlan {
                    temporary_target_bytes: 1,
                    free_headroom_bytes: 0,
                }
            ),
            Err(MaterializeError::SpacePressure)
        ));
        assert!(!destination.exists());

        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("sentinel"), b"keep").unwrap();
        assert!(matches!(
            materialize_pinned_source(&mut pin, &destination, Fixture::build_plan()),
            Err(MaterializeError::Destination(_))
        ));
        assert_eq!(fs::read(destination.join("sentinel")).unwrap(), b"keep");
        pin.close().unwrap();
    }

    #[test]
    fn accepted_source_can_have_a_valid_scene_but_an_unbuildable_declaration() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.root().join("build")).unwrap();
        fs::create_dir(fixture.root().join("game")).unwrap();
        fs::write(
            fixture.root().join("Cargo.toml"),
            b"[package]\nname='small-game'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            fixture.root().join("Cargo.lock"),
            b"# pinned fixture lock\n",
        )
        .unwrap();
        fs::write(
            fixture.root().join("rust-toolchain.toml"),
            b"[toolchain]\nchannel='1.99.1'\n",
        )
        .unwrap();
        fs::write(fixture.root().join("game/rules.rs"), b"pub fn rule() {}\n").unwrap();
        let mut paths = fixture.write("A", false, false);
        paths.extend(
            [
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
                "game/rules.rs",
                "build/headless.json",
            ]
            .map(|path| SourcePath::parse(path).unwrap()),
        );
        paths.sort_unstable();
        let declaration_path = fixture.root().join("build/headless.json");
        let valid = serde_json::to_vec(&serde_json::json!({
            "format":1,
            "cargoManifest":"Cargo.toml",
            "cargoLock":"Cargo.lock",
            "rustToolchain":"rust-toolchain.toml",
            "package":"small-game",
            "ruleSource":"game/rules.rs",
            "entryScene":SCENE_ID
        }))
        .unwrap();
        fs::write(&declaration_path, &valid).unwrap();
        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let a = fixture.admit(&mut store, 0, 1, &paths);
        let mut a_pin = store.pin_current(a.generation).unwrap();
        let a_scene = cook_entry_scene(&mut a_pin, SceneTarget::Headless).unwrap();
        let duplicate_key = String::from_utf8(valid)
            .unwrap()
            .replace("\"format\":1", "\"format\":1,\"format\":1");
        fs::write(&declaration_path, duplicate_key).unwrap();
        let declared_inputs = read_pinned_build_declaration(&mut a_pin, &a_scene).unwrap();
        assert_eq!(declared_inputs.source_root(), a);
        assert_eq!(declared_inputs.scene_product(), a_scene.revision());
        assert_eq!(declared_inputs.package(), "small-game");
        assert_eq!(declared_inputs.rule_source().as_str(), "game/rules.rs");
        a_pin.close().unwrap();

        let b = fixture.admit(&mut store, a.generation, 2, &paths);
        let mut b_pin = store.pin_current(b.generation).unwrap();
        let b_scene = cook_entry_scene(&mut b_pin, SceneTarget::Headless).unwrap();
        assert!(matches!(
            read_pinned_build_declaration(&mut b_pin, &a_scene),
            Err(BuildDeclarationError::ProductMismatch)
        ));
        assert!(matches!(
            read_pinned_build_declaration(&mut b_pin, &b_scene),
            Err(BuildDeclarationError::Syntax(_))
        ));
        b_pin.close().unwrap();

        let missing_toolchain = serde_json::to_vec(&serde_json::json!({
            "format":1,
            "cargoManifest":"Cargo.toml",
            "cargoLock":"Cargo.lock",
            "rustToolchain":"missing-toolchain.toml",
            "package":"small-game",
            "ruleSource":"game/rules.rs",
            "entryScene":SCENE_ID
        }))
        .unwrap();
        fs::write(&declaration_path, missing_toolchain).unwrap();
        let c = fixture.admit(&mut store, b.generation, 3, &paths);
        let mut c_pin = store.pin_current(c.generation).unwrap();
        let c_scene = cook_entry_scene(&mut c_pin, SceneTarget::Headless).unwrap();
        assert!(matches!(
            read_pinned_build_declaration(&mut c_pin, &c_scene),
            Err(BuildDeclarationError::MissingInput(path)) if path.as_str() == "missing-toolchain.toml"
        ));
        c_pin.close().unwrap();
        assert_eq!(declared_inputs.source_root(), a);
    }

    #[test]
    #[ignore = "local native Cargo build from the full admitted workspace"]
    fn admitted_workspace_builds_headless_game_and_failed_edit_keeps_prior_binary() {
        let mut private_cargo_home = std::env::var_os("THRUM_TEST_CARGO_HOME").map(PathBuf::from);
        let prepare_cargo_home = std::env::var_os("THRUM_TEST_PREPARE_CARGO_HOME").is_some();
        assert!(
            (private_cargo_home.is_none() && !prepare_cargo_home)
                || std::env::var_os("THRUM_TEST_NATIVE_SANDBOX").is_some(),
            "private Cargo home requires the native sandbox qualification"
        );
        assert!(
            private_cargo_home.is_none() || !prepare_cargo_home,
            "select an existing private home or prepare a new one"
        );
        let fixture = Fixture::new();
        let mut paths = fixture.write("Admitted headless game", false, false);
        let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        for root_file in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
            copy_build_input(checkout, fixture.root(), root_file, &mut paths);
        }
        for package in [
            "crates/thrum-core",
            "crates/thrum-mobile-shell",
            "crates/thrum-source",
            "crates/thrum-cook",
            "examples/i1-mobile-game",
            "tools/xtask",
        ] {
            copy_build_input(
                checkout,
                fixture.root(),
                &format!("{package}/Cargo.toml"),
                &mut paths,
            );
            copy_build_source_tree(
                checkout,
                fixture.root(),
                &format!("{package}/src"),
                &mut paths,
            );
        }
        fs::create_dir(fixture.root().join("build")).unwrap();
        fs::write(
            fixture.root().join("build/headless.json"),
            serde_json::to_vec(&serde_json::json!({
                "format":1,
                "cargoManifest":"Cargo.toml",
                "cargoLock":"Cargo.lock",
                "rustToolchain":"rust-toolchain.toml",
                "package":"i1-mobile-game",
                "ruleSource":"examples/i1-mobile-game/src/game.rs",
                "entryScene":SCENE_ID
            }))
            .unwrap(),
        )
        .unwrap();
        paths.push(SourcePath::parse("build/headless.json").unwrap());
        paths.sort_unstable();

        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let a = fixture.admit(&mut store, 0, 1, &paths);
        let rule_path = fixture.root().join("examples/i1-mobile-game/src/game.rs");
        fs::write(
            &rule_path,
            b"this working file is no longer the accepted game",
        )
        .unwrap();
        let mut a_pin = store.pin_current(a.generation).unwrap();
        let product = cook_entry_scene(&mut a_pin, SceneTarget::Headless).unwrap();
        let declaration = read_pinned_build_declaration(&mut a_pin, &product).unwrap();
        assert_eq!(declaration.source_root(), a);
        let a_input = fixture.root().join(".engine/input-a");
        let a_materialized =
            materialize_pinned_source(&mut a_pin, &a_input, Fixture::build_plan()).unwrap();
        assert_eq!(a_materialized.source_root(), a);
        let a_files = tree_hashes(a_materialized.path());
        let mut owned_cache = None;
        if prepare_cargo_home {
            let prepared =
                prepare_private_cargo_home(checkout, fixture.root(), a_materialized.path());
            private_cargo_home = Some(prepared.path.clone());
            owned_cache = Some(prepared);
        }
        let private_cache_before = private_cargo_home.as_ref().map(|path| tree_hashes(path));
        if let Some(cache) = &owned_cache {
            cache.assert_tool_bytes();
            cache.assert_reusable_source(checkout, a_materialized.path());
        }
        let a_output = fixture.root().join(".engine/output-a");
        let started = Instant::now();
        let built_a = cargo_build_headless(
            a_materialized.path(),
            &a_output,
            checkout,
            private_cargo_home.as_deref(),
        );
        let build_elapsed = started.elapsed();
        assert!(
            built_a.status.success(),
            "{}",
            String::from_utf8_lossy(&built_a.stderr)
        );
        let selected_units = private_cargo_home.as_deref().map(|home| {
            inspect_headless_cargo_units(&built_a.stdout, a_materialized.path(), home, &a_output)
        });
        let dep_info = private_cargo_home
            .as_deref()
            .map(|home| inspect_headless_dep_info(a_materialized.path(), home, &a_output));
        assert_eq!(tree_hashes(a_materialized.path()), a_files);
        let binary_name = format!("headless{}", std::env::consts::EXE_SUFFIX);
        let binary_a = a_output.join("debug").join(&binary_name);
        let binary_a_bytes = fs::read(&binary_a).unwrap();
        let binary_a_hash = Sha256::digest(&binary_a_bytes);
        let ran_a = Command::new(&binary_a).output().unwrap();
        assert!(ran_a.status.success());
        assert_eq!(
            ran_a.stdout,
            b"thrum-headless-v1 taps=1 results=1 events=1\n"
        );
        eprintln!(
            "admitted-headless-a files={} bytes={} build_ms={} binary_bytes={} binary_sha256={:02x?} selected_units={:02x?} dep_info={:02x?}",
            a_materialized.file_count(),
            a_materialized.copied_bytes(),
            build_elapsed.as_millis(),
            binary_a_bytes.len(),
            binary_a_hash,
            selected_units,
            dep_info
        );
        a_pin.close().unwrap();
        a_materialized.close().unwrap();

        fs::write(&rule_path, b"pub fn deliberately_invalid( {\n").unwrap();
        let b = fixture.admit(&mut store, a.generation, 2, &paths);
        let mut b_pin = store.pin_current(b.generation).unwrap();
        let b_product = cook_entry_scene(&mut b_pin, SceneTarget::Headless).unwrap();
        assert!(read_pinned_build_declaration(&mut b_pin, &b_product).is_ok());
        let b_input = fixture.root().join(".engine/input-b");
        let b_materialized =
            materialize_pinned_source(&mut b_pin, &b_input, Fixture::build_plan()).unwrap();
        let b_files = tree_hashes(b_materialized.path());
        if let Some(cache) = &owned_cache {
            cache.assert_reusable_source(checkout, b_materialized.path());
        }
        let b_output = fixture.root().join(".engine/output-b");
        let built_b = cargo_build_headless(
            b_materialized.path(),
            &b_output,
            checkout,
            private_cargo_home.as_deref(),
        );
        assert!(!built_b.status.success());
        let b_diagnostic = String::from_utf8_lossy(&built_b.stderr);
        assert!(b_diagnostic.contains("game.rs") && b_diagnostic.contains("error"));
        assert!(!b_output.join("debug").join(binary_name).exists());
        assert_eq!(tree_hashes(b_materialized.path()), b_files);
        assert_eq!(Sha256::digest(fs::read(&binary_a).unwrap()), binary_a_hash);
        assert!(Command::new(&binary_a).output().unwrap().status.success());
        b_pin.close().unwrap();
        b_materialized.close().unwrap();
        assert_eq!(store.current_root().unwrap().unwrap(), b);
        if let (Some(path), Some(before)) = (&private_cargo_home, private_cache_before) {
            assert_eq!(tree_hashes(path), before);
        }
        if let Some(cache) = &owned_cache {
            cache.assert_tool_bytes();
        }
        drop(owned_cache);
    }

    fn copy_build_input(from: &Path, to: &Path, relative: &str, paths: &mut Vec<SourcePath>) {
        let destination = to.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(from.join(relative), destination).unwrap();
        paths.push(SourcePath::parse(relative).unwrap());
    }

    fn copy_build_source_tree(from: &Path, to: &Path, relative: &str, paths: &mut Vec<SourcePath>) {
        for entry in fs::read_dir(from.join(relative)).unwrap() {
            let entry = entry.unwrap();
            let child = format!("{relative}/{}", entry.file_name().to_str().unwrap());
            if entry.file_type().unwrap().is_dir() {
                copy_build_source_tree(from, to, &child, paths);
            } else {
                copy_build_input(from, to, &child, paths);
            }
        }
    }

    struct PreparedCacheHome {
        path: PathBuf,
        manifest_path: PathBuf,
        target: String,
        cargo_hash: String,
        rustc_hash: String,
    }

    impl PreparedCacheHome {
        fn assert_reusable_source(&self, checkout: &Path, input: &Path) {
            let cargo = Path::new(env!("CARGO"));
            let rustc = cargo.with_file_name(format!("rustc{}", std::env::consts::EXE_SUFFIX));
            let started = Instant::now();
            let result =
                Command::new(std::env::var_os("THRUM_PYTHON").unwrap_or_else(|| "python3".into()))
                    .arg(checkout.join("scripts/prepare_headless_cargo_home.py"))
                    .arg("--source")
                    .arg(input)
                    .arg("--verify-reuse")
                    .arg(&self.manifest_path)
                    .arg("--private-home")
                    .arg(&self.path)
                    .arg("--target")
                    .arg(&self.target)
                    .arg("--cargo")
                    .arg(cargo)
                    .arg("--rustc")
                    .arg(&rustc)
                    .output()
                    .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            eprintln!(
                "verified-private-cargo-reuse ms={}",
                started.elapsed().as_millis()
            );
        }

        fn assert_tool_bytes(&self) {
            let cargo = Path::new(env!("CARGO"));
            let rustc = cargo.with_file_name(format!("rustc{}", std::env::consts::EXE_SUFFIX));
            assert_eq!(file_sha256_hex(cargo), self.cargo_hash);
            assert_eq!(file_sha256_hex(&rustc), self.rustc_hash);
        }
    }

    impl Drop for PreparedCacheHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
            let _ = fs::remove_file(&self.manifest_path);
        }
    }

    fn prepare_private_cargo_home(
        checkout: &Path,
        fixture: &Path,
        input: &Path,
    ) -> PreparedCacheHome {
        let cargo = Path::new(env!("CARGO"));
        let rustc = cargo.with_file_name(format!("rustc{}", std::env::consts::EXE_SUFFIX));
        let version = Command::new(&rustc).arg("-vV").output().unwrap();
        assert!(version.status.success());
        let details = String::from_utf8(version.stdout).unwrap();
        let target = details
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .unwrap();
        let shared_home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cargo"));
        let destination = fixture.with_extension("cargo-home");
        let manifest_path = destination.with_file_name(format!(
            "{}.manifest.json",
            destination.file_name().unwrap().to_string_lossy()
        ));
        assert!(!destination.exists() && !manifest_path.exists());
        let mut owner = PreparedCacheHome {
            path: destination.clone(),
            manifest_path: manifest_path.clone(),
            target: target.to_owned(),
            cargo_hash: String::new(),
            rustc_hash: String::new(),
        };
        let started = Instant::now();
        let result =
            Command::new(std::env::var_os("THRUM_PYTHON").unwrap_or_else(|| "python3".into()))
                .arg(checkout.join("scripts/prepare_headless_cargo_home.py"))
                .arg("--source")
                .arg(input)
                .arg("--shared-home")
                .arg(shared_home)
                .arg("--destination")
                .arg(&destination)
                .arg("--cargo")
                .arg(cargo)
                .arg("--rustc")
                .arg(&rustc)
                .arg("--target")
                .arg(target)
                .output()
                .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let candidate: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let mut packages = candidate["possibleBuildPackages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|package| package["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        packages.sort_unstable();
        assert_eq!(
            packages,
            [
                "foldhash",
                "hashbrown",
                "i1-mobile-game",
                "thrum-core",
                "thrum-mobile-shell"
            ]
        );
        owner.cargo_hash = candidate["toolExecutables"]["cargo"]["sha256"]
            .as_str()
            .unwrap()
            .to_owned();
        owner.rustc_hash = candidate["toolExecutables"]["rustc"]["sha256"]
            .as_str()
            .unwrap()
            .to_owned();
        owner.assert_tool_bytes();
        eprintln!(
            "prepared-private-cargo-home prep_ms={} {}",
            started.elapsed().as_millis(),
            String::from_utf8_lossy(&result.stdout).trim()
        );
        owner
    }

    fn tree_hashes(root: &Path) -> Vec<(PathBuf, [u8; 32])> {
        fn visit(root: &Path, current: &Path, output: &mut Vec<(PathBuf, [u8; 32])>) {
            for entry in fs::read_dir(current).unwrap() {
                let entry = entry.unwrap();
                let file_type = entry.file_type().unwrap();
                assert!(
                    !file_type.is_symlink(),
                    "build input/cache must not alias another tree"
                );
                if file_type.is_dir() {
                    visit(root, &entry.path(), output);
                } else {
                    output.push((
                        entry.path().strip_prefix(root).unwrap().to_owned(),
                        Sha256::digest(fs::read(entry.path()).unwrap()).into(),
                    ));
                }
            }
        }
        let mut output = Vec::new();
        visit(root, root, &mut output);
        output.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        output
    }

    fn file_sha256_hex(path: &Path) -> String {
        let mut file = fs::File::open(path).unwrap();
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        use std::fmt::Write as _;
        let mut hex = String::with_capacity(64);
        for byte in hash.finalize() {
            write!(&mut hex, "{byte:02x}").unwrap();
        }
        hex
    }

    fn inspect_headless_cargo_units(
        transcript: &[u8],
        input: &Path,
        cargo_home: &Path,
        output: &Path,
    ) -> [u8; 32] {
        let input = fs::canonicalize(input).unwrap();
        let cargo_home = fs::canonicalize(cargo_home).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let mut units = Vec::new();
        let mut finished = false;
        for line in transcript
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let message: serde_json::Value = serde_json::from_slice(line).unwrap();
            match message["reason"].as_str().unwrap() {
                "compiler-artifact" => {
                    let package_id = message["package_id"].as_str().unwrap();
                    let registry = package_id.starts_with("registry+");
                    let allowed_root = if registry { &cargo_home } else { &input };
                    for path in [
                        message["manifest_path"].as_str().unwrap(),
                        message["target"]["src_path"].as_str().unwrap(),
                    ] {
                        assert!(
                            fs::canonicalize(path).unwrap().starts_with(allowed_root),
                            "outside selected input: {path}"
                        );
                    }
                    let kinds = message["target"]["kind"].as_array().unwrap();
                    assert!(
                        kinds
                            .iter()
                            .all(|kind| kind != "custom-build" && kind != "proc-macro")
                    );
                    assert_eq!(message["fresh"], false, "each output tree must be new");
                    for filename in message["filenames"].as_array().unwrap() {
                        let path = Path::new(filename.as_str().unwrap());
                        assert!(
                            path.is_file() && !path.is_symlink(),
                            "missing artifact: {}",
                            path.display()
                        );
                        assert!(
                            fs::canonicalize(path).unwrap().starts_with(&output),
                            "artifact escapes output: {}",
                            path.display()
                        );
                    }
                    let executable = message["executable"].as_str();
                    if let Some(path) = executable {
                        assert!(fs::canonicalize(path).unwrap().starts_with(&output));
                    }
                    let manifest =
                        fs::canonicalize(message["manifest_path"].as_str().unwrap()).unwrap();
                    let package = if registry {
                        package_id.to_owned()
                    } else {
                        let (uri, version) = package_id.split_once('#').unwrap();
                        let package_dir = cargo_file_uri_path(uri);
                        assert_eq!(
                            fs::canonicalize(package_dir).unwrap(),
                            manifest.parent().unwrap()
                        );
                        let relative = manifest.parent().unwrap().strip_prefix(&input).unwrap();
                        format!("path:{}@{version}", relative.display())
                    };
                    let target = message["target"]["name"].as_str().unwrap();
                    let kind = kinds
                        .iter()
                        .map(|kind| kind.as_str().unwrap())
                        .collect::<Vec<_>>()
                        .join(",");
                    let features = message["features"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|feature| feature.as_str().unwrap())
                        .collect::<Vec<_>>()
                        .join(",");
                    units.push(format!("{package}:{target}:{kind}:{features}"));
                }
                "build-script-executed" => panic!("first headless profile has no build scripts"),
                "build-finished" => {
                    assert_eq!(message["success"], true);
                    assert!(!finished);
                    finished = true;
                }
                "compiler-message" => panic!("unexpected compiler diagnostic in successful build"),
                other => panic!("unrecognized Cargo build message: {other}"),
            }
        }
        assert!(finished);
        units.sort_unstable();
        assert_eq!(
            units,
            [
                "path:crates/thrum-core@0.1.0:thrum_core:lib:",
                "path:crates/thrum-mobile-shell@0.1.0:thrum_mobile_shell:lib:",
                "path:examples/i1-mobile-game@0.1.0:headless:bin:",
                "path:examples/i1-mobile-game@0.1.0:i1_mobile_game:rlib,staticlib,cdylib:",
                "registry+https://github.com/rust-lang/crates.io-index#hashbrown@0.17.1:hashbrown:lib:inline-more",
            ]
        );
        Sha256::digest(units.join("\n").as_bytes()).into()
    }

    fn inspect_headless_dep_info(input: &Path, cargo_home: &Path, output: &Path) -> [u8; 32] {
        let input = fs::canonicalize(input).unwrap();
        let cargo_home = fs::canonicalize(cargo_home).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let mut units = Vec::new();
        for entry in fs::read_dir(output.join("debug/deps")).unwrap() {
            let entry = entry.unwrap();
            if entry
                .path()
                .extension()
                .is_none_or(|extension| extension != "d")
            {
                continue;
            }
            assert!(entry.file_type().unwrap().is_file());
            assert!(entry.metadata().unwrap().len() <= 1024 * 1024);
            let bytes = fs::read(entry.path()).unwrap();
            let text = std::str::from_utf8(&bytes).unwrap();
            let first = text.lines().next().unwrap();
            let (target, dependencies) = first.split_once(": ").unwrap();
            let target = parse_dep_info_words(target);
            assert_eq!(target.len(), 1);
            assert_eq!(fs::canonicalize(&target[0]).unwrap(), entry.path());
            let dependencies = parse_dep_info_words(dependencies);
            assert!(!dependencies.is_empty());
            let mut sources = Vec::with_capacity(dependencies.len());
            for dependency in dependencies {
                let path = Path::new(&dependency);
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    input.join(path)
                };
                assert!(path.is_file() && !path.is_symlink());
                let path = fs::canonicalize(path).unwrap();
                let selected = if let Ok(relative) = path.strip_prefix(&input) {
                    format!("source:{}", relative.display())
                } else if let Ok(relative) = path.strip_prefix(&cargo_home) {
                    format!("dependency:{}", relative.display())
                } else {
                    panic!("dep-info source escapes selected roots: {}", path.display());
                };
                sources.push(selected);
            }
            sources.sort_unstable();
            sources.dedup();
            assert!(!sources.is_empty());
            units.push((entry.file_name(), sources));
        }
        assert_eq!(units.len(), 5, "unexpected rustc dep-info unit count");
        units.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let mut digest = Sha256::new();
        for (name, sources) in units {
            digest.update(name.as_encoded_bytes());
            digest.update([0]);
            for source in sources {
                digest.update(source.as_bytes());
                digest.update([0]);
            }
        }
        digest.finalize().into()
    }

    fn parse_dep_info_words(line: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut word = String::new();
        let mut chars = line.chars();
        while let Some(character) = chars.next() {
            match character {
                '\\' => {
                    assert_eq!(chars.next(), Some(' '), "unsupported dep-info escape");
                    word.push(' ');
                }
                ' ' | '\t' => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                }
                _ => word.push(character),
            }
        }
        if !word.is_empty() {
            words.push(word);
        }
        words
    }

    #[test]
    fn dep_info_words_keep_escaped_spaces_and_refuse_other_escapes() {
        assert_eq!(
            parse_dep_info_words("src/my\\ file.rs  /tmp/pkg/src/lib.rs"),
            ["src/my file.rs", "/tmp/pkg/src/lib.rs"]
        );
        assert!(std::panic::catch_unwind(|| parse_dep_info_words("src/bad\\#file.rs")).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn dep_info_audit_refuses_source_escape_malformed_rule_and_missing_unit() {
        let fixture = Fixture::new();
        let input = fixture.root().join(".engine/input");
        let cargo_home = fixture.root().join(".engine/cargo-home");
        let output = fixture.root().join(".engine/output");
        let deps = output.join("debug/deps");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&cargo_home).unwrap();
        fs::create_dir_all(&deps).unwrap();
        let source = input.join("source.rs");
        let dependency = cargo_home.join("dependency.rs");
        let outside = fixture.root().join("outside.rs");
        for path in [&source, &dependency, &outside] {
            fs::write(path, b"source").unwrap();
        }
        let depfiles = (0..5)
            .map(|index| deps.join(format!("unit-{index}.d")))
            .collect::<Vec<_>>();
        let rule = |depfile: &Path, path: &Path| {
            format!(
                "{}: {}\n",
                depfile.display().to_string().replace(' ', "\\ "),
                path.display().to_string().replace(' ', "\\ ")
            )
        };
        for depfile in &depfiles {
            fs::write(depfile, rule(depfile, &source)).unwrap();
        }
        fs::write(&depfiles[4], rule(&depfiles[4], &dependency)).unwrap();
        inspect_headless_dep_info(&input, &cargo_home, &output);

        fs::write(&depfiles[4], rule(&depfiles[4], &outside)).unwrap();
        assert!(
            std::panic::catch_unwind(|| inspect_headless_dep_info(&input, &cargo_home, &output))
                .is_err()
        );
        fs::write(&depfiles[4], b"not a dependency rule\n").unwrap();
        assert!(
            std::panic::catch_unwind(|| inspect_headless_dep_info(&input, &cargo_home, &output))
                .is_err()
        );
        fs::remove_file(&depfiles[4]).unwrap();
        assert!(
            std::panic::catch_unwind(|| inspect_headless_dep_info(&input, &cargo_home, &output))
                .is_err()
        );
    }

    fn cargo_file_uri_path(uri: &str) -> PathBuf {
        fn hex_digit(byte: u8) -> u8 {
            match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => panic!("invalid Cargo file URI escape"),
            }
        }

        let encoded = uri.strip_prefix("path+file://").unwrap().as_bytes();
        let mut decoded = Vec::with_capacity(encoded.len());
        let mut index = 0;
        while index < encoded.len() {
            if encoded[index] == b'%' {
                assert!(index + 2 < encoded.len(), "truncated Cargo file URI escape");
                decoded.push(hex_digit(encoded[index + 1]) * 16 + hex_digit(encoded[index + 2]));
                index += 3;
            } else {
                decoded.push(encoded[index]);
                index += 1;
            }
        }
        PathBuf::from(String::from_utf8(decoded).unwrap())
    }

    #[test]
    fn cargo_file_uri_decodes_spaced_unicode_path() {
        assert_eq!(
            cargo_file_uri_path("path+file:///tmp/My%20Scratch/%C3%A9"),
            PathBuf::from("/tmp/My Scratch/é")
        );
    }

    fn cargo_build_headless(
        input: &Path,
        output: &Path,
        checkout: &Path,
        private_cargo_home: Option<&Path>,
    ) -> std::process::Output {
        let sandbox = std::env::var_os("THRUM_TEST_NATIVE_SANDBOX").is_some();
        let mut command = if sandbox {
            sandboxed_cargo_command(input, output, checkout, private_cargo_home)
        } else {
            Command::new(env!("CARGO"))
        };
        command
            .current_dir(input)
            .args([
                "build",
                "--locked",
                "--offline",
                "--package",
                "i1-mobile-game",
                "--bin",
                "headless",
                "--message-format=json-render-diagnostics",
            ])
            .env("CARGO_TARGET_DIR", output)
            .env("CARGO_INCREMENTAL", "0")
            .env("RUSTC_WRAPPER", "")
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap()
    }

    #[cfg(target_os = "macos")]
    fn sandboxed_cargo_command(
        input: &Path,
        output: &Path,
        checkout: &Path,
        private_cargo_home: Option<&Path>,
    ) -> Command {
        fs::create_dir_all(output.join("tmp")).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let checkout = fs::canonicalize(checkout).unwrap();
        let input = fs::canonicalize(input).unwrap();
        let cargo_home = private_cargo_home
            .map(Path::to_path_buf)
            .unwrap_or_else(|| {
                std::env::var_os("CARGO_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cargo")
                    })
            });
        let cargo_home = fs::canonicalize(cargo_home).unwrap();
        let toolchain =
            fs::canonicalize(Path::new(env!("CARGO")).parent().unwrap().parent().unwrap()).unwrap();
        let developer = std::env::var_os("DEVELOPER_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let selected = Command::new("/usr/bin/xcode-select")
                    .arg("-p")
                    .output()
                    .unwrap();
                assert!(selected.status.success());
                PathBuf::from(String::from_utf8(selected.stdout).unwrap().trim_end())
            });
        let developer = if developer.join("Contents/Developer").is_dir() {
            developer.join("Contents/Developer")
        } else {
            developer
        };
        let developer = fs::canonicalize(developer).unwrap();
        let xcrun_path = |args: &[&str]| {
            let found = Command::new("/usr/bin/xcrun")
                .env("DEVELOPER_DIR", &developer)
                .env_remove("SDKROOT")
                .env_remove("TOOLCHAINS")
                .args(args)
                .output()
                .unwrap();
            assert!(found.status.success());
            let path = String::from_utf8(found.stdout).unwrap();
            let path = fs::canonicalize(path.trim_end()).unwrap();
            assert!(path.starts_with(&developer));
            path
        };
        let linker = xcrun_path(&["--sdk", "macosx", "--find", "clang"]);
        let sdk = xcrun_path(&["--sdk", "macosx", "--show-sdk-path"]);
        let linker_root = linker
            .ancestors()
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "xctoolchain")
            })
            .unwrap_or(&developer);
        let profile = output.join("build.sb");
        fs::write(
            &profile,
            "(version 1)\n(deny default)\n(deny network*)\n(allow process*)\n(allow mach-lookup)\n(allow sysctl-read)\n(allow file-read-metadata)\n(allow file-read* (literal \"/\") (subpath \"/System/Library\") (subpath \"/System/Cryptexes\") (subpath \"/usr/lib\") (subpath \"/usr/bin\") (subpath \"/Library/Preferences\") (subpath \"/private/etc\") (subpath \"/dev\") (subpath (param \"INPUT\")) (subpath (param \"OUTPUT\")) (subpath (param \"CARGO_HOME\")) (subpath (param \"TOOLCHAIN\")) (subpath (param \"LINKER_ROOT\")) (subpath (param \"SDK\")))\n(allow file-write* (subpath (param \"OUTPUT\")) (literal \"/dev/null\"))\n",
        )
        .unwrap();
        let paths = SandboxPaths {
            input: &input,
            output: &output,
            cargo_home: &cargo_home,
            toolchain: &toolchain,
            linker_root,
            sdk: &sdk,
        };
        let blocked_read = sandbox_command(&profile, paths, "/bin/cat")
            .arg(checkout.join("Cargo.toml"))
            .output()
            .unwrap();
        assert!(!blocked_read.status.success());
        let blocked_project = sandbox_command(&profile, paths, "/bin/cat")
            .arg(input.parent().unwrap().parent().unwrap().join("Cargo.toml"))
            .output()
            .unwrap();
        assert!(!blocked_project.status.success());
        let readable_system_file = Path::new("/private/etc/hosts");
        assert!(readable_system_file.is_file());
        let allowed_system = sandbox_command(&profile, paths, "/bin/cat")
            .arg(readable_system_file)
            .output()
            .unwrap();
        assert!(
            allowed_system.status.success(),
            "{}",
            String::from_utf8_lossy(&allowed_system.stderr)
        );
        assert_eq!(
            allowed_system.stdout,
            fs::read(readable_system_file).unwrap()
        );
        let mut system_read_probes = 0;
        for denied in [
            Path::new("/usr/share/misc/ascii"),
            Path::new("/System/Applications/Calculator.app/Contents/Info.plist"),
        ] {
            if denied.is_file() {
                let blocked_system = sandbox_command(&profile, paths, "/bin/cat")
                    .arg(denied)
                    .output()
                    .unwrap();
                assert!(!blocked_system.status.success(), "{}", denied.display());
                system_read_probes += 1;
            }
        }
        assert!(system_read_probes > 0, "no system read boundary was probed");
        let blocked_write = sandbox_command(&profile, paths, "/usr/bin/touch")
            .arg(input.join("should-not-exist"))
            .output()
            .unwrap();
        assert!(!blocked_write.status.success());
        if private_cargo_home.is_some() {
            let shared_config =
                PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cargo/config.toml");
            if shared_config.is_file() && !shared_config.starts_with(&cargo_home) {
                let blocked_shared = sandbox_command(&profile, paths, "/bin/cat")
                    .arg(shared_config)
                    .output()
                    .unwrap();
                assert!(!blocked_shared.status.success());
            }
        }
        let mut command = sandbox_command(&profile, paths, env!("CARGO"));
        let rustc_version = Command::new(toolchain.join("bin/rustc"))
            .arg("-vV")
            .output()
            .unwrap();
        assert!(rustc_version.status.success());
        let rustc_version = String::from_utf8(rustc_version.stdout).unwrap();
        let host = rustc_version
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .unwrap();
        assert!(host.ends_with("-apple-darwin"));
        let linker_variable = format!(
            "CARGO_TARGET_{}_LINKER",
            host.to_ascii_uppercase().replace('-', "_")
        );
        command
            .env_clear()
            .env("HOME", cargo_home.parent().unwrap())
            .env("CARGO_HOME", &cargo_home)
            .env("RUSTC", toolchain.join("bin/rustc"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", toolchain.join("bin").display()),
            )
            .env("RUSTFLAGS", "")
            .env(linker_variable, linker)
            .env("SDKROOT", sdk)
            .env("LC_ALL", "C")
            .env("TMPDIR", output.join("tmp"));
        command
    }

    #[cfg(not(target_os = "macos"))]
    fn sandboxed_cargo_command(
        _input: &Path,
        _output: &Path,
        _checkout: &Path,
        _private_cargo_home: Option<&Path>,
    ) -> Command {
        panic!("macOS sandbox qualification only");
    }

    #[cfg(target_os = "macos")]
    #[derive(Clone, Copy)]
    struct SandboxPaths<'a> {
        input: &'a Path,
        output: &'a Path,
        cargo_home: &'a Path,
        toolchain: &'a Path,
        linker_root: &'a Path,
        sdk: &'a Path,
    }

    #[cfg(target_os = "macos")]
    fn sandbox_command(profile: &Path, paths: SandboxPaths<'_>, program: &str) -> Command {
        fn parameter(name: &str, path: &Path) -> OsString {
            let mut value = OsString::from(name);
            value.push("=");
            value.push(path.as_os_str());
            value
        }
        let mut command = Command::new("/usr/bin/sandbox-exec");
        command
            .arg("-D")
            .arg(parameter("INPUT", paths.input))
            .arg("-D")
            .arg(parameter("OUTPUT", paths.output))
            .arg("-D")
            .arg(parameter("CARGO_HOME", paths.cargo_home))
            .arg("-D")
            .arg(parameter("TOOLCHAIN", paths.toolchain))
            .arg("-D")
            .arg(parameter("LINKER_ROOT", paths.linker_root))
            .arg("-D")
            .arg(parameter("SDK", paths.sdk))
            .arg("-f")
            .arg(profile)
            .arg(program);
        command
    }

    #[test]
    #[ignore = "local release-only pinned scene cook cost probe"]
    fn measure_pinned_small_scene_cook() {
        let fixture = Fixture::new();
        let paths = fixture.write("Small headless scene", false, false);
        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let root = fixture.admit(&mut store, 0, 1, &paths);
        let mut pin = store.pin_current(root.generation).unwrap();
        let mut times = Vec::with_capacity(1_000);
        for _ in 0..100 {
            std::hint::black_box(cook_entry_scene(&mut pin, SceneTarget::Headless).unwrap());
        }
        for _ in 0..1_000 {
            let started = Instant::now();
            std::hint::black_box(cook_entry_scene(&mut pin, SceneTarget::Headless).unwrap());
            times.push(started.elapsed().as_micros());
        }
        times.sort_unstable();
        eprintln!(
            "pinned small-scene cook: 1000 samples, p50={}us p95={}us p99={}us max={}us",
            times[500], times[950], times[990], times[999]
        );
        pin.close().unwrap();
    }

    #[test]
    #[ignore = "local release-only pinned source materialization cost probe"]
    fn measure_small_pinned_source_materialization() {
        let fixture = Fixture::new();
        let paths = fixture.write("Small build input", false, false);
        let mut store = NativeSourceStore::open(
            &fixture.root().join(".engine/source.sqlite"),
            Fixture::plan(),
        )
        .unwrap();
        let root = fixture.admit(&mut store, 0, 1, &paths);
        let mut pin = store.pin_current(root.generation).unwrap();
        let mut times = Vec::with_capacity(200);
        for index in 0..220 {
            let destination = fixture.root().join(format!(".engine/materialize-{index}"));
            let started = Instant::now();
            let output =
                materialize_pinned_source(&mut pin, &destination, Fixture::build_plan()).unwrap();
            let elapsed = started.elapsed().as_micros();
            output.close().unwrap();
            if index >= 20 {
                times.push(elapsed);
            }
        }
        times.sort_unstable();
        eprintln!(
            "pinned 3-file materialization: 200 samples, p50={}us p95={}us p99={}us max={}us",
            times[100], times[190], times[198], times[199]
        );
        pin.close().unwrap();
    }
}
