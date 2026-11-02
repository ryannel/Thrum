//! First F4 headless scene product from an accepted, pinned source root.
//!
//! This product is not a runnable build: no game-code or toolchain closure is
//! bound yet. Cooking never opens the mutable working tree.

use sha2::{Digest, Sha256};
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
    let mut digest = Sha256::new();
    digest.update(COOK_FORMAT);
    digest.update([match target {
        SceneTarget::Headless => 1,
    }]);
    // Only declared direct inputs enter the cook key. An unrelated admitted
    // file may advance SourceRevision without invalidating this product.
    digest.update(input_hash(pin, &project_path)?);
    digest.update(input_hash(pin, &scene_path)?);
    Ok(SceneProduct {
        root,
        target,
        revision: SceneProductRevision(digest.finalize().into()),
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
    use std::fs;
    use std::path::{Path, PathBuf};
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
        a_pin.close().unwrap();

        let b_paths = fixture.write("B", false, false);
        let b = fixture.admit(&mut store, a.generation, 2, &b_paths);
        let mut b_pin = store.pin_current(b.generation).unwrap();
        let b_product = cook_entry_scene(&mut b_pin, SceneTarget::Headless).unwrap();
        assert_eq!(b_product.label(), Some("B"));
        assert_ne!(b_product.revision(), prior.revision());
        b_pin.close().unwrap();

        let c_paths = fixture.write("B", false, true);
        let c = fixture.admit(&mut store, b.generation, 3, &c_paths);
        let mut c_pin = store.pin_current(c.generation).unwrap();
        let c_product = cook_entry_scene(&mut c_pin, SceneTarget::Headless).unwrap();
        assert_ne!(c_product.source_root(), b_product.source_root());
        assert_eq!(c_product.revision(), b_product.revision());
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
}
