use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use fs4::FileExt;
use thrum_cook::{BuildScratchPlan, materialize_pinned_source};
use thrum_source::{
    NativeSourceStore, SourcePath, SourceSpacePlan, StageSpacePressure,
    registry_stage::validate_staged_candidate, stage_declared_tree_with_plan,
};

const MIB: u64 = 1024 * 1024;
const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const HISTORY_ID: &str = "6a5ef644-7225-4770-9f4d-e1d9c0a7d515";

fn allocated_file_bytes(root: &Path) -> u64 {
    let mut total = 0;
    let mut directories = vec![root.to_owned()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                directories.push(entry.path());
            } else {
                total += File::open(entry.path()).unwrap().allocated_size().unwrap();
            }
        }
    }
    total
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("thrum-t2-space-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("scenes")).unwrap();
        fs::create_dir(root.join("schemas")).unwrap();
        fs::create_dir(root.join(".engine")).unwrap();
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }

    fn scene(&self, caption: &str) {
        fs::write(
            self.root().join("scenes/main.scene.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":SCENE_ID,"type":"core.scene","schema":1,
                "fields":{"scene.label":caption}
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn main() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root().join("project.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":PROJECT_ID,"type":"core.project","schema":2,
            "fields":{"project.format":1,
                "project.records":["scenes/main.scene.json"],
                "project.schemas":[],
                "project.registryHistory":"schemas/history.registry.json",
                "project.entryScenes":[SCENE_ID]}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        fixture.root().join("schemas/history.registry.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":HISTORY_ID,"type":"core.registry-history","schema":1,
            "fields":{"registry.fields":[],"registry.versions":[]}
        }))
        .unwrap(),
    )
    .unwrap();
    fixture.scene("A");
    let mut master = File::create(fixture.root().join("master.bin")).unwrap();
    let block = [0x5a_u8; 4096];
    for _ in 0..(12 * MIB / block.len() as u64) {
        master.write_all(&block).unwrap();
    }
    drop(master);
    let paths = [
        "master.bin",
        "project.json",
        "scenes/main.scene.json",
        "schemas/history.registry.json",
    ]
    .map(|path| SourcePath::parse(path).unwrap());
    let plan = SourceSpacePlan {
        main_target_bytes: 416 * MIB,
        wal_stop_bytes: 16 * MIB,
        temporary_target_bytes: 16 * MIB,
        free_headroom_bytes: 64 * MIB,
    };
    let scratch = BuildScratchPlan {
        temporary_target_bytes: 16 * MIB,
        free_headroom_bytes: 64 * MIB,
    };
    let store = NativeSourceStore::open(&fixture.root().join(".engine/source.sqlite"), plan).unwrap();
    let stage_a = stage_declared_tree_with_plan(fixture.root(), &paths, plan).unwrap();
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    drop(candidate_a);
    drop(stage_a);
    let mut a_pin = store.pin_current(a.generation).unwrap();
    let old_scene = fs::read(fixture.root().join("scenes/main.scene.json")).unwrap();
    let available_before_f4 = fs4::available_space(fixture.root()).unwrap();
    let first = materialize_pinned_source(
        &mut a_pin,
        &fixture.root().join(".engine/build-inputs-a"),
        scratch,
    )
    .unwrap();
    let second = materialize_pinned_source(
        &mut a_pin,
        &fixture.root().join(".engine/build-inputs-b"),
        scratch,
    )
    .unwrap();
    assert_eq!(first.source_root(), a);
    assert_eq!(second.source_root(), a);
    let copied_each = first.copied_bytes();
    let first_allocated = allocated_file_bytes(first.path());
    let second_allocated = allocated_file_bytes(second.path());
    let available_with_two = fs4::available_space(fixture.root()).unwrap();
    assert!(copied_each > 12 * MIB);
    assert_eq!(copied_each, second.copied_bytes());
    assert!(copied_each * 2 > scratch.temporary_target_bytes);
    fixture.scene("B");
    let two_refusal = match stage_declared_tree_with_plan(fixture.root(), &paths, plan) {
        Ok(_) => panic!("two retained F4 trees unexpectedly permitted B capture"),
        Err(error) => error,
    };
    let Some(StageSpacePressure::TemporaryScan {
        charged_at_least_bytes,
        target_bytes,
        ..
    }) = two_refusal.space_pressure() else {
        panic!("expected existing temporary scan refusal: {two_refusal:?}");
    };
    assert_eq!(target_bytes, 16 * MIB);
    assert!(charged_at_least_bytes > target_bytes);
    assert_eq!(store.current_root().unwrap(), Some(a));
    assert!(first.path().exists() && second.path().exists());
    for path in &paths {
        let expected = if path.as_str() == "scenes/main.scene.json" {
            old_scene.clone()
        } else {
            fs::read(fixture.root().join(path.as_str())).unwrap()
        };
        assert_eq!(fs::read(first.path().join(path.as_str())).unwrap(), expected);
        assert_eq!(fs::read(second.path().join(path.as_str())).unwrap(), expected);
    }
    let relaxed = SourceSpacePlan {
        temporary_target_bytes: 64 * MIB,
        ..plan
    };
    let relaxed_stage = stage_declared_tree_with_plan(fixture.root(), &paths, relaxed).unwrap();
    let relaxed_candidate = validate_staged_candidate(&relaxed_stage, &[]).unwrap();
    let expected_b_revision = relaxed_candidate.provisional_manifest().unwrap().revision();
    drop(relaxed_candidate);
    drop(relaxed_stage);

    first.close().unwrap();
    let available_with_one = fs4::available_space(fixture.root()).unwrap();
    let one_refusal = match stage_declared_tree_with_plan(fixture.root(), &paths, plan) {
        Ok(_) => panic!("one retained F4 tree unexpectedly permitted B capture"),
        Err(error) => error,
    };
    let Some(StageSpacePressure::TemporaryPreflight {
        existing_charge_bytes,
        projected_stage_bytes,
        target_bytes,
    }) = one_refusal.space_pressure() else {
        panic!("expected existing plus stage projection refusal: {one_refusal:?}");
    };
    assert_eq!(target_bytes, 16 * MIB);
    assert!(existing_charge_bytes + projected_stage_bytes > target_bytes);
    assert_eq!(store.current_root().unwrap(), Some(a));
    assert_eq!(
        fs::read(second.path().join("scenes/main.scene.json")).unwrap(),
        old_scene
    );
    second.close().unwrap();
    let available_after_f4 = fs4::available_space(fixture.root()).unwrap();
    let stage_b = stage_declared_tree_with_plan(fixture.root(), &paths, plan).unwrap();
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    assert_eq!(candidate_b.provisional_manifest().unwrap().revision(), expected_b_revision);
    let b = store.admit(a.generation, 1, 2, [2; 32], &candidate_b).unwrap();
    assert_eq!(store.current_root().unwrap(), Some(b));
    let mut copied = Vec::new();
    a_pin
        .copy_file_to(&SourcePath::parse("scenes/main.scene.json").unwrap(), &mut copied)
        .unwrap();
    assert_eq!(copied, old_scene);
    a_pin.close().unwrap();
    println!(
        "two_materializations copied_each={} aggregate_copied={} first_allocated={} second_allocated={} first_refusal_at_least={} one_existing={} one_projected={} target={} available_before_f4={} available_with_two={} available_with_one={} available_after_f4={} a_generation={} b_generation={} b_revision_policy_stable=true f4_bytes_unchanged=true old_pin_exact=true",
        copied_each,
        copied_each * 2,
        first_allocated,
        second_allocated,
        charged_at_least_bytes,
        existing_charge_bytes,
        projected_stage_bytes,
        target_bytes,
        available_before_f4,
        available_with_two,
        available_with_one,
        available_after_f4,
        a.generation,
        b.generation,
    );
}
