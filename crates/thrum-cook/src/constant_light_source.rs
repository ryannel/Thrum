//! Selected constant-light cooking from one authenticated, accepted source pin.

use thrum_source::{
    AcceptedSourceRoot, MAX_RECORD_BYTES, ManifestEntry, PinnedSource, SourcePath,
    SourceStoreError, parse_authored_environment, parse_environment_light_identity,
    parse_environment_scene,
    project_v2_json::{ProjectV2, parse_project_v2},
};

use crate::{ConstantLightError, ConstantLightInputV1, ConstantLightProductV1};

/// The four records in the selected constant-light source closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstantLightRecordKind {
    /// `core.project@2` at `project.json`.
    Project,
    /// The sole declared entry's `core.scene@2`.
    Scene,
    /// Its `thrum.environment.authored@1`.
    Environment,
    /// The source Sun's `thrum.light.identity@1`.
    Light,
}

impl ConstantLightRecordKind {
    fn record_type(self) -> &'static str {
        match self {
            Self::Project => "core.project",
            Self::Scene => "core.scene",
            Self::Environment => "thrum.environment.authored",
            Self::Light => "thrum.light.identity",
        }
    }

    fn schema(self) -> u64 {
        match self {
            Self::Project | Self::Scene => 2,
            Self::Environment | Self::Light => 1,
        }
    }
}

/// Exact authenticated input identity, after successful copy and typed decoding.
/// This is provenance for later binding; it defines no persistent key grammar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstantLightRecordInput {
    kind: ConstantLightRecordKind,
    id: String,
    path: SourcePath,
    length: u64,
    sha256: [u8; 32],
}

impl ConstantLightRecordInput {
    /// Selected record's role in the source closure.
    pub fn kind(&self) -> ConstantLightRecordKind {
        self.kind
    }

    /// Stable record UUID, checked against the authenticated manifest.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Record type checked by both manifest selection and typed decoding.
    pub fn record_type(&self) -> &'static str {
        self.kind.record_type()
    }

    /// Schema checked by the fixed typed decoder.
    pub fn schema(&self) -> u64 {
        self.kind.schema()
    }

    /// Authenticated portable source path; selected records are project-declared.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Full source file length and SHA-256, including opaque fields and labels.
    pub fn content_identity(&self) -> (u64, [u8; 32]) {
        (self.length, self.sha256)
    }
}

/// Pure product and exact admitted provenance of its selected source closure.
/// This does not publish a product, select a run or authorize a ready build.
#[derive(Debug)]
pub struct PinnedConstantLight {
    root: AcceptedSourceRoot,
    records: [ConstantLightRecordInput; 4],
    input: ConstantLightInputV1,
    product: ConstantLightProductV1,
}

impl PinnedConstantLight {
    /// Exact accepted source root used for every selected read.
    pub fn source_root(&self) -> AcceptedSourceRoot {
        self.root
    }

    /// Verified inputs, ordered project, scene, environment, light.
    pub fn records(&self) -> &[ConstantLightRecordInput; 4] {
        &self.records
    }

    /// Checked authored numeric values before normalization and zero canonicalization.
    pub fn input(&self) -> &ConstantLightInputV1 {
        &self.input
    }

    /// Immutable normalized 160-byte CPU product.
    pub fn product(&self) -> &ConstantLightProductV1 {
        &self.product
    }
}

/// A selected accepted source cannot produce the bounded constant-light product.
#[derive(Debug)]
pub enum PinnedConstantLightError {
    /// A full-file verified copy failed. Any partial output is discarded.
    Source(SourceStoreError),
    /// The fixed project or a selected ID/type/path is absent or undeclared.
    Selection(ConstantLightRecordKind),
    /// A selected record exceeds the bounded source parser's byte limit.
    RecordTooLarge(ConstantLightRecordKind),
    /// Fixed schema decoding failed, including source numeric tolerances.
    Decode(ConstantLightRecordKind),
    /// Decoded ID differs from the authenticated manifest/selected reference.
    Identity(ConstantLightRecordKind),
    /// This cooker requires exactly one project entry scene.
    EntryCount,
    /// The selected scene has no environment source.
    EnvironmentNotSelected,
    /// Pure product numeric/identity checks failed after source closure validation.
    Product(ConstantLightError),
}

/// Cook the sole entry scene's constant light using only one accepted pin.
///
/// Every selected record must agree with both the authenticated manifest and
/// project declaration. Bytes are decoded only after `copy_file_to` verifies
/// the complete file. No mutable checkout, unselected source bytes, persistent
/// cache or active product is accessed; an error leaves any held prior result
/// and the caller's pin available. Source labels/opaque fields remain outside
/// the numeric payload but are covered by exact input provenance.
pub fn cook_pinned_constant_light(
    pin: &mut PinnedSource,
) -> Result<PinnedConstantLight, PinnedConstantLightError> {
    use ConstantLightRecordKind::{Environment, Light, Project, Scene};
    use PinnedConstantLightError as Error;

    let project_path = SourcePath::parse("project.json").expect("fixed portable path");
    let project_entry = pin
        .manifest()
        .files()
        .binary_search_by(|file| file.path().cmp(&project_path))
        .ok()
        .map(|index| &pin.manifest().files()[index])
        .filter(|entry| {
            entry.record_identity() == Some((pin.manifest().project_id(), Project.record_type()))
        })
        .ok_or(Error::Selection(Project))?;
    let project_input = provenance(project_entry, Project);
    let project =
        parse_project_v2(&read_record(pin, &project_input)?).map_err(|_| Error::Decode(Project))?;
    check_id(project.id(), &project_input)?;
    let [scene_id] = project.entry_scenes() else {
        return Err(Error::EntryCount);
    };
    let scene_input = selected_input(pin, &project, scene_id, Scene)?;
    let scene = parse_environment_scene(&read_record(pin, &scene_input)?)
        .map_err(|_| Error::Decode(Scene))?;
    check_id(scene.id(), &scene_input)?;
    let environment_id = scene
        .environment_source()
        .ok_or(Error::EnvironmentNotSelected)?;
    let environment_input = selected_input(pin, &project, environment_id, Environment)?;
    let environment = parse_authored_environment(&read_record(pin, &environment_input)?)
        .map_err(|_| Error::Decode(Environment))?;
    check_id(environment.id(), &environment_input)?;
    let sun = environment.sun();
    let light_input = selected_input(pin, &project, sun.light_ref(), Light)?;
    let light = parse_environment_light_identity(&read_record(pin, &light_input)?)
        .map_err(|_| Error::Decode(Light))?;
    check_id(light.id(), &light_input)?;
    let input = ConstantLightInputV1 {
        environment_id: uuid_bytes(environment.id()),
        light_id: uuid_bytes(light.id()),
        toward_source_unit: sun.toward_source_unit(),
        unit_luminance_linear_rec709: sun.unit_luminance_linear_rec709(),
        perpendicular_illuminance_lux: sun.perpendicular_illuminance_lux(),
        angular_radius_rad: sun.angular_radius_rad(),
        ambient_irradiance_lux: environment.ambient_irradiance_lux(),
        background_radiance_cd_m2: environment.background_radiance_cd_m2(),
    };
    let product = ConstantLightProductV1::cook(input).map_err(Error::Product)?;
    Ok(PinnedConstantLight {
        root: pin.root(),
        records: [project_input, scene_input, environment_input, light_input],
        input,
        product,
    })
}

fn provenance(entry: &ManifestEntry, kind: ConstantLightRecordKind) -> ConstantLightRecordInput {
    let (length, sha256) = entry.content_identity();
    ConstantLightRecordInput {
        kind,
        id: entry
            .record_identity()
            .expect("selected record identity")
            .0
            .to_owned(),
        path: entry.path().clone(),
        length,
        sha256,
    }
}

fn selected_input(
    pin: &PinnedSource,
    project: &ProjectV2,
    id: &str,
    kind: ConstantLightRecordKind,
) -> Result<ConstantLightRecordInput, PinnedConstantLightError> {
    // A fixed three scans avoid retaining another whole-manifest index.
    let entry = pin
        .manifest()
        .files()
        .iter()
        .find(|entry| entry.record_identity() == Some((id, kind.record_type())))
        .filter(|entry| project.record_paths().binary_search(entry.path()).is_ok())
        .ok_or(PinnedConstantLightError::Selection(kind))?;
    Ok(provenance(entry, kind))
}

fn read_record(
    pin: &mut PinnedSource,
    input: &ConstantLightRecordInput,
) -> Result<Vec<u8>, PinnedConstantLightError> {
    let capacity = usize::try_from(input.length)
        .ok()
        .filter(|length| *length <= MAX_RECORD_BYTES)
        .ok_or(PinnedConstantLightError::RecordTooLarge(input.kind))?;
    let mut bytes = Vec::with_capacity(capacity);
    pin.copy_file_to(&input.path, &mut bytes)
        .map_err(PinnedConstantLightError::Source)?;
    Ok(bytes)
}

fn check_id(id: &str, input: &ConstantLightRecordInput) -> Result<(), PinnedConstantLightError> {
    if id == input.id {
        Ok(())
    } else {
        Err(PinnedConstantLightError::Identity(input.kind))
    }
}

fn uuid_bytes(id: &str) -> [u8; 16] {
    // Fixed source parsers already established canonical lowercase UUID syntax.
    let mut digits = id.bytes().filter(|byte| *byte != b'-');
    let nibble = |byte: u8| match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("typed source parser checked UUID"),
    };
    std::array::from_fn(|_| {
        let high = digits.next().expect("checked UUID length");
        let low = digits.next().expect("checked UUID length");
        (nibble(high) << 4) | nibble(low)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use thrum_source::{
        NativeSourceStore, SourceSpacePlan, SourceStoreErrorKind,
        registry_stage::validate_staged_candidate, stage_declared_tree_with_plan,
    };

    const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
    const SCENE: &str = "10000000-0000-4000-8000-000000000002";
    const ENVIRONMENT: &str = "10000000-0000-4000-8000-000000000003";
    const LIGHT: &str = "10000000-0000-4000-8000-000000000004";
    const HISTORY: &str = "10000000-0000-4000-8000-000000000005";
    const SECOND_SCENE: &str = "10000000-0000-4000-8000-000000000006";
    const ENVIRONMENT_PATH: &str = "records/environment.record.json";
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "thrum-constant-light-cook-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&root).unwrap();
            for directory in ["records", "scenes", "schemas", ".engine"] {
                fs::create_dir(root.join(directory)).unwrap();
            }
            Self(root)
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

        fn store(&self) -> NativeSourceStore {
            NativeSourceStore::open(&self.root().join(".engine/source.sqlite"), Self::plan())
                .unwrap()
        }

        fn write_json(&self, path: &str, value: Value) {
            fs::write(self.root().join(path), serde_json::to_vec(&value).unwrap()).unwrap();
        }

        fn edit_json(&self, path: &str, edit: impl FnOnce(&mut Value)) {
            let mut value =
                serde_json::from_slice(&fs::read(self.root().join(path)).unwrap()).unwrap();
            edit(&mut value);
            self.write_json(path, value);
        }

        fn write(&self, lux: u64, unrelated: &str) -> Vec<SourcePath> {
            self.write_json(
                "project.json",
                json!({
                    "id":PROJECT,"type":"core.project","schema":2,
                    "fields":{
                        "project.format":1,
                        "project.records":[ENVIRONMENT_PATH,"records/light.record.json","scenes/main.scene.json"],
                        "project.schemas":[],"project.registryHistory":"schemas/history.registry.json",
                        "project.entryScenes":[SCENE]
                    }
                }),
            );
            self.write_json(
                "scenes/main.scene.json",
                json!({"id":SCENE,"type":"core.scene","schema":2,
                    "fields":{"scene.label":"A","environment.source":ENVIRONMENT}}),
            );
            self.write_json(
                ENVIRONMENT_PATH,
                json!({"id":ENVIRONMENT,"type":"thrum.environment.authored","schema":1,
                    "fields":{
                        "profile":"constant-light-v1",
                        "sun":{
                            "light_ref":LIGHT,"toward_source_unit":[-0.6,0.8000000001,-0.0],
                            "perpendicular_illuminance_lux":lux,
                            "unit_luminance_linear_rec709":[0.5,1.0,2.47229917],
                            "angular_radius_rad":0.00465,"direction_model":"geometric-straight",
                            "applied_attenuation":[]
                        },
                        "ambient_irradiance_lux":[1,2,3],"background_radiance_cd_m2":[4,5,6],
                        "unsupported":["wind","cloud_tau","precipitation","air","thermal","moon","air_quality"],
                        "x.thrum.environment.note":{"opaque":true}
                    }
                }),
            );
            self.write_json(
                "records/light.record.json",
                json!({"id":LIGHT,"type":"thrum.light.identity","schema":1,
                    "fields":{"driver_binding":"incoming-environment-sun","label":"Sun"}}),
            );
            self.write_json(
                "schemas/history.registry.json",
                json!({"id":HISTORY,"type":"core.registry-history","schema":1,
                    "fields":{"registry.fields":[],"registry.versions":[]}}),
            );
            fs::write(self.root().join("note.txt"), unrelated).unwrap();
            [
                "project.json",
                ENVIRONMENT_PATH,
                "records/light.record.json",
                "scenes/main.scene.json",
                "schemas/history.registry.json",
                "note.txt",
            ]
            .map(|path| SourcePath::parse(path).unwrap())
            .to_vec()
        }

        fn admit(
            &self,
            store: &NativeSourceStore,
            expected: i64,
            key: i64,
            paths: &[SourcePath],
        ) -> AcceptedSourceRoot {
            let stage = stage_declared_tree_with_plan(self.root(), paths, Self::plan()).unwrap();
            let candidate = validate_staged_candidate(&stage, &[]).unwrap();
            store
                .admit(expected, 1, key, [key as u8; 32], &candidate)
                .unwrap()
        }

        fn fault_blob(&self, hash: [u8; 32], fault: &str) {
            // Python is a contributor prerequisite; no production SQLite bypass
            // or additional Rust dependency is exposed for fault injection.
            let output = Command::new(
                std::env::var_os("THRUM_PYTHON").unwrap_or_else(|| "python3".into()),
            )
            .arg("-c")
            .arg(
                "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1])\nh=bytes.fromhex(sys.argv[2])\nf=sys.argv[3]\ni=1 if f=='partial' else 0\nb=c.execute('SELECT bytes FROM blob_chunks WHERE hash=? AND chunk_index=?',(h,i)).fetchone()[0]\nb=(b[:-1] if f in ('truncate','partial') else b.replace(b'100000',b'200000',1))\nassert c.execute('UPDATE blob_chunks SET bytes=? WHERE hash=? AND chunk_index=?',(b,h,i)).rowcount==1\nc.commit()",
            )
            .arg(self.root().join(".engine/source.sqlite"))
            .arg(hash.iter().map(|byte| format!("{byte:02x}")).collect::<String>())
            .arg(fault)
            .output()
            .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn accepted_a_cooks_after_b_and_checkout_edits_without_losing_a_on_b_refusal() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let paths = fixture.write(100_000, "A");
        let a = fixture.admit(&store, 0, 1, &paths);
        let mut a_pin = store.pin_accepted(a).unwrap();
        let prior = cook_pinned_constant_light(&mut a_pin).unwrap();
        assert_eq!(prior.source_root(), a);
        let expected_input = ConstantLightInputV1 {
            environment_id: [0x10, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 3],
            light_id: [0x10, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 4],
            toward_source_unit: [-0.6, 0.8000000001, -0.0],
            unit_luminance_linear_rec709: [0.5, 1.0, 2.47229917],
            perpendicular_illuminance_lux: 100_000.0,
            angular_radius_rad: 0.00465,
            ambient_irradiance_lux: [1.0, 2.0, 3.0],
            background_radiance_cd_m2: [4.0, 5.0, 6.0],
        };
        assert_eq!(prior.input(), &expected_input);
        assert!(prior.input().toward_source_unit[2].is_sign_negative());
        assert_eq!(
            prior.product(),
            &ConstantLightProductV1::cook(expected_input).unwrap()
        );
        for (input, (kind, id, path, record_type, schema)) in prior.records().iter().zip([
            (
                ConstantLightRecordKind::Project,
                PROJECT,
                "project.json",
                "core.project",
                2,
            ),
            (
                ConstantLightRecordKind::Scene,
                SCENE,
                "scenes/main.scene.json",
                "core.scene",
                2,
            ),
            (
                ConstantLightRecordKind::Environment,
                ENVIRONMENT,
                ENVIRONMENT_PATH,
                "thrum.environment.authored",
                1,
            ),
            (
                ConstantLightRecordKind::Light,
                LIGHT,
                "records/light.record.json",
                "thrum.light.identity",
                1,
            ),
        ]) {
            assert_eq!(
                (
                    input.kind(),
                    input.id(),
                    input.path().as_str(),
                    input.record_type(),
                    input.schema()
                ),
                (kind, id, path, record_type, schema)
            );
            let admitted = a_pin
                .manifest()
                .files()
                .iter()
                .find(|file| file.path() == input.path())
                .unwrap();
            assert_eq!(input.content_identity(), admitted.content_identity());
        }

        let paths = fixture.write(75_000, "B");
        let b = fixture.admit(&store, a.generation, 2, &paths);
        let mut b_pin = store.pin_accepted(b).unwrap();
        let b_product = cook_pinned_constant_light(&mut b_pin).unwrap();
        assert_eq!(b_product.input().perpendicular_illuminance_lux, 75_000.0);
        assert_ne!(b_product.product().to_bytes(), prior.product().to_bytes());
        for path in [
            "project.json",
            "scenes/main.scene.json",
            ENVIRONMENT_PATH,
            "records/light.record.json",
        ] {
            fs::write(fixture.root().join(path), b"broken mutable checkout").unwrap();
        }
        assert_eq!(
            cook_pinned_constant_light(&mut a_pin).unwrap().product(),
            prior.product()
        );
        assert_eq!(
            cook_pinned_constant_light(&mut b_pin).unwrap().product(),
            b_product.product()
        );

        let paths = fixture.write(50_000, "C");
        fixture.edit_json("scenes/main.scene.json", |scene| {
            scene["fields"]
                .as_object_mut()
                .unwrap()
                .remove("environment.source");
        });
        let c = fixture.admit(&store, b.generation, 3, &paths);
        let mut c_pin = store.pin_accepted(c).unwrap();
        assert!(matches!(
            cook_pinned_constant_light(&mut c_pin),
            Err(PinnedConstantLightError::EnvironmentNotSelected)
        ));
        assert_eq!(store.current_root().unwrap(), Some(c));
        assert_eq!(
            cook_pinned_constant_light(&mut a_pin).unwrap().product(),
            prior.product()
        );
        assert_eq!(prior.source_root(), a);
        c_pin.close().unwrap();
        b_pin.close().unwrap();
        a_pin.close().unwrap();
    }

    #[test]
    fn unrelated_bytes_and_opaque_edits_keep_product_but_preserve_exact_provenance() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let paths = fixture.write(100_000, "A");
        let a = fixture.admit(&store, 0, 1, &paths);
        let mut pin = store.pin_accepted(a).unwrap();
        let first = cook_pinned_constant_light(&mut pin).unwrap();
        pin.close().unwrap();
        fs::write(fixture.root().join("note.txt"), "unrelated source edit").unwrap();
        let b = fixture.admit(&store, a.generation, 2, &paths);
        let mut pin = store.pin_accepted(b).unwrap();
        let second = cook_pinned_constant_light(&mut pin).unwrap();
        assert_ne!(first.source_root().revision, second.source_root().revision);
        assert_eq!(first.records(), second.records());
        assert_eq!(first.product().to_bytes(), second.product().to_bytes());
        pin.close().unwrap();
        fixture.edit_json(ENVIRONMENT_PATH, |environment| {
            environment["fields"]["x.thrum.environment.note"] = json!({"opaque":"changed"});
            environment["fields"]["label"] = json!("new source caption");
        });
        let c = fixture.admit(&store, b.generation, 3, &paths);
        let mut pin = store.pin_accepted(c).unwrap();
        let third = cook_pinned_constant_light(&mut pin).unwrap();
        assert_eq!(first.input(), third.input());
        assert_eq!(first.product().to_bytes(), third.product().to_bytes());
        assert_ne!(
            first.records()[2].content_identity(),
            third.records()[2].content_identity()
        );
        pin.close().unwrap();
    }

    #[test]
    fn selected_entry_requires_schema_two_and_exactly_one_entry() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let mut paths = fixture.write(100_000, "A");
        fixture.edit_json("scenes/main.scene.json", |scene| {
            scene["schema"] = json!(1);
            scene["fields"]
                .as_object_mut()
                .unwrap()
                .remove("environment.source");
        });
        let a = fixture.admit(&store, 0, 1, &paths);
        let mut pin = store.pin_accepted(a).unwrap();
        assert!(matches!(
            cook_pinned_constant_light(&mut pin),
            Err(PinnedConstantLightError::Decode(
                ConstantLightRecordKind::Scene
            ))
        ));
        pin.close().unwrap();
        fixture.write_json(
            "scenes/second.scene.json",
            json!({
                "id":SECOND_SCENE,"type":"core.scene","schema":2,"fields":{}
            }),
        );
        fixture.edit_json("project.json", |project| {
            project["fields"]["project.records"]
                .as_array_mut()
                .unwrap()
                .push(json!("scenes/second.scene.json"));
            project["fields"]["project.entryScenes"]
                .as_array_mut()
                .unwrap()
                .push(json!(SECOND_SCENE));
        });
        paths.push(SourcePath::parse("scenes/second.scene.json").unwrap());
        let b = fixture.admit(&store, a.generation, 2, &paths);
        let mut pin = store.pin_accepted(b).unwrap();
        assert!(matches!(
            cook_pinned_constant_light(&mut pin),
            Err(PinnedConstantLightError::EntryCount)
        ));
        pin.close().unwrap();
    }

    #[test]
    fn selected_references_require_manifest_type_project_path_and_exact_identity() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let paths = fixture.write(100_000, "A");
        let a = fixture.admit(&store, 0, 1, &paths);
        let mut pin = store.pin_accepted(a).unwrap();
        let prior = cook_pinned_constant_light(&mut pin).unwrap();
        let project =
            parse_project_v2(&fs::read(fixture.root().join("project.json")).unwrap()).unwrap();
        for (id, kind) in [
            (ENVIRONMENT, ConstantLightRecordKind::Scene),
            (LIGHT, ConstantLightRecordKind::Environment),
            (SCENE, ConstantLightRecordKind::Light),
            (SECOND_SCENE, ConstantLightRecordKind::Scene),
        ] {
            assert!(matches!(selected_input(&pin, &project, id, kind),
                Err(PinnedConstantLightError::Selection(refused)) if refused == kind));
        }
        // These altered declarations cannot pass F3 admission. Exercise the
        // cooker's own defense against an undeclared authenticated record too.
        fixture.edit_json("project.json", |project| {
            project["fields"]["project.records"] = json!(["scenes/main.scene.json"]);
        });
        let incomplete_project =
            parse_project_v2(&fs::read(fixture.root().join("project.json")).unwrap()).unwrap();
        for (id, kind) in [
            (ENVIRONMENT, ConstantLightRecordKind::Environment),
            (LIGHT, ConstantLightRecordKind::Light),
        ] {
            assert!(
                matches!(selected_input(&pin, &incomplete_project, id, kind),
                Err(PinnedConstantLightError::Selection(refused)) if refused == kind)
            );
        }
        for input in prior.records() {
            assert!(matches!(check_id(SECOND_SCENE, input),
                Err(PinnedConstantLightError::Identity(kind)) if kind == input.kind()));
        }
        for (scene_ref, light_ref) in [(LIGHT, LIGHT), (ENVIRONMENT, SCENE)] {
            fixture.write(100_000, "invalid replacement");
            fixture.edit_json("scenes/main.scene.json", |scene| {
                scene["fields"]["environment.source"] = json!(scene_ref);
            });
            fixture.edit_json(ENVIRONMENT_PATH, |environment| {
                environment["fields"]["sun"]["light_ref"] = json!(light_ref);
            });
            let stage =
                stage_declared_tree_with_plan(fixture.root(), &paths, Fixture::plan()).unwrap();
            assert!(validate_staged_candidate(&stage, &[]).is_err());
            assert_eq!(store.current_root().unwrap(), Some(a));
            assert_eq!(
                cook_pinned_constant_light(&mut pin).unwrap().product(),
                prior.product()
            );
        }
        pin.close().unwrap();
    }

    #[test]
    fn corrupt_complete_and_truncated_selected_copies_refuse_before_decode() {
        for fault in ["corrupt", "truncate"] {
            let fixture = Fixture::new();
            let store = fixture.store();
            let paths = fixture.write(100_000, "A");
            let a = fixture.admit(&store, 0, 1, &paths);
            let mut pin = store.pin_accepted(a).unwrap();
            let prior = cook_pinned_constant_light(&mut pin).unwrap();
            let hash = prior.records()[2].content_identity().1;
            fixture.fault_blob(hash, fault);
            // The corrupt replacement is valid JSON and valid source quantities;
            // only whole-file authentication distinguishes it from admitted A.
            let error = cook_pinned_constant_light(&mut pin).unwrap_err();
            let PinnedConstantLightError::Source(error) = error else {
                panic!("copy failure must precede source decode: {error:?}");
            };
            assert_eq!(error.kind(), SourceStoreErrorKind::CorruptStore);
            assert_eq!(prior.input().perpendicular_illuminance_lux, 100_000.0);
            assert_eq!(prior.source_root(), a);
            assert_eq!(store.current_root().unwrap(), Some(a));
            pin.close().unwrap();
        }
    }

    #[test]
    fn second_chunk_failure_discards_partial_source_without_decoding() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let paths = fixture.write(100_000, "A");
        let mut bytes = fs::read(fixture.root().join(ENVIRONMENT_PATH)).unwrap();
        // Legal trailing whitespace crosses the production store's 4 MiB
        // chunk boundary without adding opaque nodes or source meanings.
        const FIRST_CHUNK_BYTES: usize = 4 * 1024 * 1024;
        bytes.resize(FIRST_CHUNK_BYTES + 1, b' ');
        fs::write(fixture.root().join(ENVIRONMENT_PATH), &bytes).unwrap();
        drop(bytes);
        let a = fixture.admit(&store, 0, 1, &paths);
        let mut pin = store.pin_accepted(a).unwrap();
        let prior = cook_pinned_constant_light(&mut pin).unwrap();
        fixture.fault_blob(prior.records()[2].content_identity().1, "partial");
        let mut partial = Vec::new();
        let copy_error = pin
            .copy_file_to(prior.records()[2].path(), &mut partial)
            .unwrap_err();
        assert_eq!(copy_error.kind(), SourceStoreErrorKind::CorruptStore);
        assert_eq!(partial.len(), FIRST_CHUNK_BYTES);
        // Even this prefix is valid source JSON; it must never authorize cook.
        assert!(parse_authored_environment(&partial).is_ok());
        drop(partial);
        assert!(matches!(cook_pinned_constant_light(&mut pin),
            Err(PinnedConstantLightError::Source(error))
                if error.kind() == SourceStoreErrorKind::CorruptStore));
        assert_eq!(prior.input().perpendicular_illuminance_lux, 100_000.0);
        assert_eq!(prior.source_root(), a);
        assert_eq!(store.current_root().unwrap(), Some(a));
        pin.close().unwrap();
    }
}
