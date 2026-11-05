//! Constant-light presentation product from one accepted source pin.

use sha2::{Digest, Sha256};
use thrum_content_runtime::{
    EnvironmentProduct as RuntimeEnvironmentProduct, EnvironmentProductRevision, EnvironmentValues,
};
use thrum_source::{
    AcceptedSourceRoot, PinnedSource, SourcePath, SourceStoreError, parse_authored_environment,
    parse_environment_light_identity, parse_environment_scene, project_v2_json::parse_project_v2,
};

use crate::{RecordReadError, input_hash, read_record};

const FORMAT: &[u8] = b"thrum.environment-product.v1\0";
const VALUE_COUNT: usize = 14;
const ARTIFACT_BYTES: usize = FORMAT.len() + 32 + 4 * 32 + VALUE_COUNT * 8;

/// Immutable source-bound constant-light product.
pub struct EnvironmentProduct {
    root: AcceptedSourceRoot,
    revision: EnvironmentProductRevision,
    inputs: [[u8; 32]; 4],
    values: EnvironmentValues,
}

impl EnvironmentProduct {
    /// Accepted source root used to prepare this instance.
    pub fn source_root(&self) -> AcceptedSourceRoot {
        self.root
    }

    /// Reusable direct-input product identity.
    pub fn revision(&self) -> EnvironmentProductRevision {
        self.revision
    }

    /// Checked photometric quantities.
    pub fn values(&self) -> EnvironmentValues {
        self.values
    }

    /// Drop source-store provenance when handing checked values to a running game.
    pub fn runtime_product(&self) -> RuntimeEnvironmentProduct {
        RuntimeEnvironmentProduct::new(self.revision, self.values)
    }

    /// Encode deterministic bytes for a separately authenticated manifest.
    pub fn artifact(&self) -> EnvironmentArtifact {
        let mut bytes = Vec::with_capacity(ARTIFACT_BYTES);
        bytes.extend_from_slice(FORMAT);
        bytes.extend_from_slice(self.revision.as_bytes());
        for hash in self.inputs {
            bytes.extend_from_slice(&hash);
        }
        for number in self.values.numbers() {
            bytes.extend_from_slice(&number.to_le_bytes());
        }
        EnvironmentArtifact {
            content_hash: Sha256::digest(&bytes).into(),
            bytes,
        }
    }
}

/// Encoded product and its independent content digest.
pub struct EnvironmentArtifact {
    bytes: Vec<u8>,
    content_hash: [u8; 32],
}

impl EnvironmentArtifact {
    /// Deterministic product encoding.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// SHA-256 of the encoding.
    pub fn content_hash(&self) -> [u8; 32] {
        self.content_hash
    }
}

/// The selected authored closure cannot be cooked.
#[derive(Debug)]
pub enum EnvironmentCookError {
    /// Verified pinned read failed.
    Source(SourceStoreError),
    /// Required pinned record is absent or exceeds the parser bound.
    Record,
    /// Project declaration or identity is invalid.
    Project,
    /// Exactly one entry scene is required.
    EntryCount,
    /// A required scene, environment, or light edge is missing or ambiguous.
    Reference,
    /// A typed record is invalid or its declared identity differs.
    Value,
}

impl From<RecordReadError> for EnvironmentCookError {
    fn from(error: RecordReadError) -> Self {
        match error {
            RecordReadError::Source(error) => Self::Source(error),
            RecordReadError::Missing | RecordReadError::TooLarge => Self::Record,
        }
    }
}

/// Cook one selected constant-light closure from an accepted read pin.
///
/// # Errors
///
/// Returns a typed error if any pinned record, identity or edge is invalid.
///
/// # Panics
///
/// Panics only if the fixed project path violates the internal path parser.
pub fn cook_render_environment(
    pin: &mut PinnedSource,
) -> Result<EnvironmentProduct, EnvironmentCookError> {
    let root = pin.root();
    let project_path = SourcePath::parse("project.json").expect("fixed portable path");
    let project = parse_project_v2(&read_record(pin, &project_path)?)
        .map_err(|_| EnvironmentCookError::Project)?;
    if project.id() != pin.manifest().project_id() {
        return Err(EnvironmentCookError::Project);
    }
    let [scene_id] = project.entry_scenes() else {
        return Err(EnvironmentCookError::EntryCount);
    };
    let scene_path = record_path(pin, &project, scene_id, "core.scene")?;
    let scene = parse_environment_scene(&read_record(pin, &scene_path)?)
        .map_err(|_| EnvironmentCookError::Value)?;
    if scene.id() != scene_id || scene.schema() != 2 {
        return Err(EnvironmentCookError::Value);
    }
    let environment_id = scene
        .environment_source()
        .ok_or(EnvironmentCookError::Reference)?;
    let environment_path =
        record_path(pin, &project, environment_id, "thrum.environment.authored")?;
    let environment = parse_authored_environment(&read_record(pin, &environment_path)?)
        .map_err(|_| EnvironmentCookError::Value)?;
    if environment.id() != environment_id {
        return Err(EnvironmentCookError::Value);
    }
    let light_path = record_path(
        pin,
        &project,
        environment.sun().light_ref(),
        "thrum.light.identity",
    )?;
    let light = parse_environment_light_identity(&read_record(pin, &light_path)?)
        .map_err(|_| EnvironmentCookError::Value)?;
    if light.id() != environment.sun().light_ref() {
        return Err(EnvironmentCookError::Value);
    }
    let sun = environment.sun();
    let direction = sun.toward_source_unit();
    let colour = sun.unit_luminance_linear_rec709();
    let ambient = environment.ambient_irradiance_lux();
    let background = environment.background_radiance_cd_m2();
    let values = EnvironmentValues::from_numbers([
        direction[0],
        direction[1],
        direction[2],
        sun.perpendicular_illuminance_lux(),
        colour[0],
        colour[1],
        colour[2],
        sun.angular_radius_rad(),
        ambient[0],
        ambient[1],
        ambient[2],
        background[0],
        background[1],
        background[2],
    ])
    .ok_or(EnvironmentCookError::Value)?;
    let inputs = [
        input_hash(pin, &project_path)?,
        input_hash(pin, &scene_path)?,
        input_hash(pin, &environment_path)?,
        input_hash(pin, &light_path)?,
    ];
    Ok(EnvironmentProduct {
        root,
        revision: derive_revision(inputs),
        inputs,
        values,
    })
}

fn record_path(
    pin: &PinnedSource,
    project: &thrum_source::project_v2_json::ProjectV2,
    id: &str,
    kind: &str,
) -> Result<SourcePath, EnvironmentCookError> {
    let mut matches = pin
        .manifest()
        .files()
        .iter()
        .filter(|file| file.record_identity() == Some((id, kind)));
    let path = matches
        .next()
        .map(|file| file.path().clone())
        .ok_or(EnvironmentCookError::Reference)?;
    if matches.next().is_some() || project.record_paths().binary_search(&path).is_err() {
        return Err(EnvironmentCookError::Reference);
    }
    Ok(path)
}

fn derive_revision(inputs: [[u8; 32]; 4]) -> EnvironmentProductRevision {
    let mut digest = Sha256::new();
    digest.update(FORMAT);
    for hash in inputs {
        digest.update(hash);
    }
    EnvironmentProductRevision::from_bytes(digest.finalize().into())
}

/// Serialized bytes disagree with the trusted manifest or product format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentLoadError {
    /// Encoded bytes differ from the separately recorded digest.
    ContentHash,
    /// The encoding or checked numeric payload is invalid.
    Format,
    /// Direct-input product key differs from the expected key.
    Revision,
}

/// Reconstitute a product after separately verifying its trusted manifest.
///
/// # Errors
///
/// Returns an error on hash, format, numeric or product-key mismatch.
pub fn load_environment_artifact(
    bytes: &[u8],
    expected_content_hash: [u8; 32],
    expected_root: AcceptedSourceRoot,
    expected_revision: EnvironmentProductRevision,
) -> Result<EnvironmentProduct, EnvironmentLoadError> {
    if bytes.len() != ARTIFACT_BYTES || !bytes.starts_with(FORMAT) {
        return Err(EnvironmentLoadError::Format);
    }
    if Sha256::digest(bytes).as_slice() != expected_content_hash {
        return Err(EnvironmentLoadError::ContentHash);
    }
    let mut cursor = &bytes[FORMAT.len()..];
    let revision = take::<32>(&mut cursor)?;
    let inputs = [
        take::<32>(&mut cursor)?,
        take::<32>(&mut cursor)?,
        take::<32>(&mut cursor)?,
        take::<32>(&mut cursor)?,
    ];
    if revision != *expected_revision.as_bytes() || derive_revision(inputs) != expected_revision {
        return Err(EnvironmentLoadError::Revision);
    }
    let mut numbers = [0.0; VALUE_COUNT];
    for number in &mut numbers {
        *number = f64::from_le_bytes(take::<8>(&mut cursor)?);
    }
    if !cursor.is_empty() {
        return Err(EnvironmentLoadError::Format);
    }
    let values = EnvironmentValues::from_numbers(numbers).ok_or(EnvironmentLoadError::Format)?;
    Ok(EnvironmentProduct {
        root: expected_root,
        revision: expected_revision,
        inputs,
        values,
    })
}

fn take<const N: usize>(cursor: &mut &[u8]) -> Result<[u8; N], EnvironmentLoadError> {
    let Some((head, tail)) = cursor.split_first_chunk::<N>() else {
        return Err(EnvironmentLoadError::Format);
    };
    *cursor = tail;
    Ok(*head)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use thrum_source::{
        NativeSourceStore, SourceSpacePlan, registry_stage::validate_staged_candidate,
        stage_declared_tree_with_plan,
    };

    use super::*;

    const PROJECT: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const ENVIRONMENT: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const LIGHT: &str = "5a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HISTORY: &str = "6a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn artifact_numeric_gate_accepts_f3_tolerance_boundary() {
        let values = EnvironmentValues::from_numbers([
            0.0,
            1.0,
            0.0,
            100_000.0,
            0.088_462_886_047_803_61,
            0.060_368_322_438_510_39,
            12.991_930_293_881_085,
            0.004_65,
            1000.0,
            1000.0,
            1000.0,
            0.0,
            0.0,
            0.0,
        ])
        .expect("F3 tolerance boundary");
        assert_eq!(values.toward_sun(), [0.0, 1.0, 0.0]);
    }

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let number = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "thrum-environment-cook-{}-{number}",
                std::process::id()
            ));
            for directory in ["scenes", "records", "schemas", ".engine"] {
                fs::create_dir_all(path.join(directory)).unwrap();
            }
            Self(path)
        }

        fn plan() -> SourceSpacePlan {
            SourceSpacePlan {
                main_target_bytes: 1 << 30,
                wal_stop_bytes: 1 << 26,
                temporary_target_bytes: 1 << 26,
                free_headroom_bytes: 0,
            }
        }

        fn write(&self, sun_lux: u32, unrelated: bool, environment_edge: bool) -> Vec<SourcePath> {
            let records = if environment_edge {
                vec![
                    "records/environment.record.json",
                    "records/light.record.json",
                    "scenes/main.scene.json",
                ]
            } else {
                vec!["scenes/main.scene.json"]
            };
            fs::write(
                self.0.join("project.json"),
                serde_json::to_vec(&serde_json::json!({
                    "id": PROJECT, "type":"core.project", "schema":2,
                    "fields":{"project.format":1,"project.records":records,
                        "project.schemas":[],"project.registryHistory":"schemas/history.registry.json",
                        "project.entryScenes":[SCENE]}
                })).unwrap(),
            ).unwrap();
            let fields = if environment_edge {
                serde_json::json!({"environment.source": ENVIRONMENT})
            } else {
                serde_json::json!({})
            };
            fs::write(
                self.0.join("scenes/main.scene.json"),
                serde_json::to_vec(&serde_json::json!({
                    "id":SCENE,"type":"core.scene","schema":2,"fields":fields
                }))
                .unwrap(),
            )
            .unwrap();
            let mut paths = vec![
                "project.json",
                "scenes/main.scene.json",
                "schemas/history.registry.json",
            ];
            if environment_edge {
                fs::write(
                    self.0.join("records/environment.record.json"),
                    serde_json::to_vec(&serde_json::json!({
                        "id":ENVIRONMENT,"type":"thrum.environment.authored","schema":1,
                        "fields":{"profile":"constant-light-v1",
                            "sun":{"light_ref":LIGHT,"toward_source_unit":[0,1,0],
                                "perpendicular_illuminance_lux":sun_lux,
                                "unit_luminance_linear_rec709":[1,1,1],
                                "angular_radius_rad":0.00465,
                                "direction_model":"geometric-straight","applied_attenuation":[]},
                            "ambient_irradiance_lux":[1000,1000,1000],
                            "background_radiance_cd_m2":[0,0,0],
                            "unsupported":["wind","cloud_tau","precipitation","air","thermal","moon","air_quality"]}
                    })).unwrap(),
                ).unwrap();
                fs::write(
                    self.0.join("records/light.record.json"),
                    serde_json::to_vec(&serde_json::json!({
                        "id":LIGHT,"type":"thrum.light.identity","schema":1,
                        "fields":{"driver_binding":"incoming-environment-sun"}
                    }))
                    .unwrap(),
                )
                .unwrap();
                paths.extend([
                    "records/environment.record.json",
                    "records/light.record.json",
                ]);
            } else {
                for record in ["environment.record.json", "light.record.json"] {
                    let path = self.0.join("records").join(record);
                    if path.exists() {
                        fs::remove_file(path).unwrap();
                    }
                }
            }
            fs::write(
                self.0.join("schemas/history.registry.json"),
                serde_json::to_vec(&serde_json::json!({
                    "id":HISTORY,"type":"core.registry-history","schema":1,
                    "fields":{"registry.fields":[],"registry.versions":[]}
                }))
                .unwrap(),
            )
            .unwrap();
            if unrelated {
                fs::write(self.0.join("notes.txt"), "unrelated").unwrap();
                paths.push("notes.txt");
            } else if self.0.join("notes.txt").exists() {
                fs::remove_file(self.0.join("notes.txt")).unwrap();
            }
            paths.sort_unstable();
            paths
                .into_iter()
                .map(|path| SourcePath::parse(path).unwrap())
                .collect()
        }

        fn admit(
            &self,
            store: &NativeSourceStore,
            old: i64,
            key: i64,
            paths: &[SourcePath],
        ) -> AcceptedSourceRoot {
            let stage = stage_declared_tree_with_plan(&self.0, paths, Self::plan()).unwrap();
            let candidate = validate_staged_candidate(&stage, &[]).unwrap();
            store
                .admit(old, 1, key, [u8::try_from(key).unwrap(); 32], &candidate)
                .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn accepted_closure_cooks_a_b_and_reuses_unrelated_edit() {
        let fixture = Fixture::new();
        let store =
            NativeSourceStore::open(&fixture.0.join(".engine/source.sqlite"), Fixture::plan())
                .unwrap();
        let a_paths = fixture.write(100_000, false, true);
        let a = fixture.admit(&store, 0, 1, &a_paths);
        let mut a_pin = store.pin_current(a.generation).unwrap();
        let product_a = cook_render_environment(&mut a_pin).unwrap();
        assert_eq!(product_a.source_root(), a);
        assert_eq!(product_a.values().sun_lux(), 100_000.0);
        let runtime = product_a.runtime_product();
        assert_eq!(runtime.revision(), product_a.revision());
        assert_eq!(runtime.values(), product_a.values());
        let artifact = product_a.artifact();
        let loaded = load_environment_artifact(
            artifact.bytes(),
            artifact.content_hash(),
            a,
            product_a.revision(),
        )
        .unwrap();
        assert_eq!(loaded.values(), product_a.values());
        let mut damaged = artifact.bytes().to_vec();
        *damaged.last_mut().unwrap() ^= 1;
        assert_eq!(
            load_environment_artifact(&damaged, artifact.content_hash(), a, product_a.revision())
                .err()
                .unwrap(),
            EnvironmentLoadError::ContentHash
        );
        let mut invalid_numeric = artifact.bytes().to_vec();
        let first_value = FORMAT.len() + 32 + 4 * 32;
        invalid_numeric[first_value..first_value + 8].copy_from_slice(&f64::NAN.to_le_bytes());
        assert_eq!(
            load_environment_artifact(
                &invalid_numeric,
                Sha256::digest(&invalid_numeric).into(),
                a,
                product_a.revision()
            )
            .err()
            .unwrap(),
            EnvironmentLoadError::Format
        );
        a_pin.close().unwrap();

        let b_paths = fixture.write(80_000, false, true);
        let b = fixture.admit(&store, a.generation, 2, &b_paths);
        let mut b_pin = store.pin_current(b.generation).unwrap();
        let product_b = cook_render_environment(&mut b_pin).unwrap();
        assert_ne!(product_b.revision(), product_a.revision());
        assert_eq!(product_b.values().sun_lux(), 80_000.0);
        b_pin.close().unwrap();

        let c_paths = fixture.write(80_000, true, true);
        let c = fixture.admit(&store, b.generation, 3, &c_paths);
        let mut c_pin = store.pin_current(c.generation).unwrap();
        let product_c = cook_render_environment(&mut c_pin).unwrap();
        assert_ne!(product_c.source_root(), product_b.source_root());
        assert_eq!(product_c.revision(), product_b.revision());
        assert_eq!(product_c.artifact().bytes(), product_b.artifact().bytes());
        c_pin.close().unwrap();

        let d_paths = fixture.write(80_000, false, false);
        let d = fixture.admit(&store, c.generation, 4, &d_paths);
        let mut d_pin = store.pin_current(d.generation).unwrap();
        assert!(matches!(
            cook_render_environment(&mut d_pin),
            Err(EnvironmentCookError::Reference)
        ));
        assert_eq!(product_b.values().sun_lux(), 80_000.0);
        d_pin.close().unwrap();
    }
}
