//! Retained, unready output from the first local native-build qualification.
//! The digest of this report is diagnostic; it is never a `BuildRevision`.

use crate::{SceneProduct, load_scene_artifact};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thrum_source::AcceptedSourceRoot;

const REPORT: &str = "candidate.json";
const SCENE: &str = "scene.bin";
const BINARY: &str = "headless";
const MAX_FILES: usize = 128;
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_REPORT_BYTES: u64 = 32 * 1024;
const MAX_JSON_BYTES: u64 = 128 * 1024;
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);
const EVIDENCE: [&str; 9] = [
    "build.sb",
    "build-declaration.json",
    "cargo-supervisor-request.json",
    "cargo-supervisor.json",
    "cargo-transcript.jsonl",
    "environment.json",
    "private-cache-manifest.json",
    "receipt-verification.json",
    "rustc-receipt-wrapper",
];

/// Named source evidence is copied along with the executable. Names are fixed
/// by the caller and checked against a small flat-path vocabulary.
pub(crate) struct EvidenceFile<'a> {
    pub name: &'static str,
    pub source: &'a Path,
}

pub(crate) struct CandidateInput<'a> {
    pub root: AcceptedSourceRoot,
    pub product: &'a SceneProduct,
    pub binary: &'a Path,
    pub receipts: &'a Path,
    pub evidence: &'a [EvidenceFile<'a>],
    pub cargo_succeeded: bool,
}

pub(crate) fn hex(bytes: impl AsRef<[u8]>) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in bytes.as_ref() {
        write!(&mut output, "{byte:02x}").expect("string write");
    }
    output
}

fn ordinary_file(path: &Path) -> io::Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate file is not ordinary",
        ));
    }
    Ok(metadata)
}

fn read_bounded(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let metadata = ordinary_file(path)?;
    if metadata.len() > limit {
        return Err(invalid("candidate file exceeds read limit"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(invalid("candidate file changed while reading"));
    }
    Ok(bytes)
}

fn stamp(path: &Path, limit: u64) -> io::Result<(u64, String)> {
    let metadata = ordinary_file(path)?;
    if metadata.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate file exceeds limit",
        ));
    }
    let mut input = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut length = 0u64;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length += count as u64;
        if length > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "candidate file grew",
            ));
        }
        digest.update(&buffer[..count]);
    }
    if length != metadata.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate file changed length",
        ));
    }
    Ok((length, hex(digest.finalize())))
}

fn copy_stamped(source: &Path, destination: &Path) -> io::Result<serde_json::Value> {
    let source_metadata = ordinary_file(source)?;
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut length = 0u64;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length += count as u64;
        if length > MAX_FILE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "candidate file exceeds limit",
            ));
        }
        output.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
    }
    fs::set_permissions(destination, source_metadata.permissions())?;
    output.sync_all()?;
    let hash = hex(digest.finalize());
    if stamp(destination, MAX_FILE_BYTES)? != (length, hash.clone()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "candidate copy changed",
        ));
    }
    Ok(serde_json::json!({"bytes":length,"sha256":hash}))
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 40
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
        })
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_small_json(path: &Path) -> io::Result<serde_json::Value> {
    serde_json::from_slice(&read_bounded(path, MAX_JSON_BYTES)?).map_err(invalid_json)
}

/// Fill an owned, new and empty staging directory with a checked local build.
/// The caller has already checked Cargo's status, units, dep-info and receipt
/// reconciliation. This layer retains their evidence and refuses incomplete
/// copies; it cannot close compiler reads or authenticate release origins.
pub(crate) fn retain(path: &Path, input: CandidateInput<'_>) -> io::Result<String> {
    if !input.cargo_succeeded
        || input.product.source_root() != input.root
        || input.evidence.len() != EVIDENCE.len()
    {
        return Err(invalid(
            "candidate inputs are not a successful matching build",
        ));
    }
    let stage = fs::symlink_metadata(path)?;
    if !stage.is_dir() || stage.file_type().is_symlink() || fs::read_dir(path)?.next().is_some() {
        return Err(invalid(
            "candidate stage is not an empty ordinary directory",
        ));
    }
    let mut files = BTreeMap::new();
    let scene = input.product.artifact();
    let scene_path = path.join(SCENE);
    let mut scene_output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&scene_path)?;
    scene_output.write_all(scene.bytes())?;
    scene_output.sync_all()?;
    let scene_stamp = stamp(&scene_path, MAX_FILE_BYTES)?;
    if scene_stamp.1 != hex(scene.content_hash()) {
        return Err(invalid("scene copy changed"));
    }
    files.insert(
        SCENE.to_owned(),
        serde_json::json!({"bytes":scene_stamp.0,"sha256":scene_stamp.1}),
    );
    files.insert(
        BINARY.to_owned(),
        copy_stamped(input.binary, &path.join(BINARY))?,
    );
    for evidence in input.evidence {
        if !EVIDENCE.contains(&evidence.name) || files.contains_key(evidence.name) {
            return Err(invalid("invalid candidate evidence name"));
        }
        files.insert(
            evidence.name.to_owned(),
            copy_stamped(evidence.source, &path.join(evidence.name))?,
        );
    }
    let receipt_metadata = fs::symlink_metadata(input.receipts)?;
    if !receipt_metadata.is_dir() || receipt_metadata.file_type().is_symlink() {
        return Err(invalid("receipt directory is not ordinary"));
    }
    let mut receipt_count = 0;
    for entry in fs::read_dir(input.receipts)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("non-UTF8 receipt name"))?;
        if !name.ends_with(".receipt")
            || name.len() <= 8
            || !name[..name.len() - 8]
                .bytes()
                .all(|byte| byte.is_ascii_digit())
            || name.len() > 24
        {
            return Err(invalid("invalid receipt name"));
        }
        receipt_count += 1;
        if receipt_count > 64 || files.len() + 1 > MAX_FILES {
            return Err(invalid("too many candidate files"));
        }
        if ordinary_file(&entry.path())?.len() > 1024 * 1024 {
            return Err(invalid("receipt exceeds limit"));
        }
        files.insert(name.clone(), copy_stamped(&entry.path(), &path.join(name))?);
    }
    if receipt_count == 0 {
        return Err(invalid("empty receipt set"));
    }
    let report = serde_json::json!({
        "format":1,"kind":"local-native-candidate","buildReady":false,
        "sourceGeneration":input.root.generation,
        "sourceRevision":hex(input.root.revision.as_bytes()),
        "productRevision":hex(input.product.revision().as_bytes()),
        "target":"headless-macos-arm64-dev-v1","receiptCount":receipt_count,
        "files":files
    });
    let encoded = serde_json::to_vec(&report).map_err(invalid_json)?;
    if encoded.len() as u64 > MAX_REPORT_BYTES {
        return Err(invalid("candidate report exceeds limit"));
    }
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.join(REPORT))?;
    output.write_all(&encoded)?;
    output.sync_all()?;
    let digest = hex(Sha256::digest(encoded));
    verify(path, &digest, input.root, input.product)?;
    Ok(digest)
}

/// Install a verified, unready bundle as one directory entry. The parent is
/// exclusively owned by the caller: no other writer may claim `destination`.
/// A killed writer can leave only an ignored staging directory before rename;
/// a failed parent sync can leave a visible but unselected candidate.
pub(crate) fn publish(destination: &Path, input: CandidateInput<'_>) -> io::Result<String> {
    publish_directory(destination, |stage| retain(stage, input))
}

fn publish_directory<T>(
    destination: &Path,
    build: impl FnOnce(&Path) -> io::Result<T>,
) -> io::Result<T> {
    if !destination.is_absolute() || destination.file_name().is_none() {
        return Err(invalid("candidate destination must be absolute"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("candidate destination lacks parent"))?;
    let parent_metadata = fs::symlink_metadata(parent)?;
    if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(invalid("candidate parent is not an ordinary directory"));
    }
    if fs::symlink_metadata(destination).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "candidate destination exists",
        ));
    }
    let mut stage = None;
    for _ in 0..64 {
        let sequence = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".thrum-candidate-stage-{}-{sequence}",
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => {
                stage = Some(candidate);
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    let stage = stage.ok_or_else(|| invalid("candidate staging names exhausted"))?;
    let result = (|| {
        let result = build(&stage)?;
        fs::File::open(&stage)?.sync_all()?;
        if fs::symlink_metadata(destination).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "candidate destination appeared during staging",
            ));
        }
        fs::rename(&stage, destination)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(result)
    })();
    if result.is_err() && stage.exists() {
        let _ = fs::remove_dir_all(stage);
    }
    result
}

/// Reload using only retained adjacent files. `expected_product` supplies the
/// accepted source association and product key; no source or compiler tree is
/// consulted. The report itself is never a ready build manifest.
pub(crate) fn verify(
    path: &Path,
    expected_report_sha256: &str,
    root: AcceptedSourceRoot,
    expected_product: &SceneProduct,
) -> io::Result<PathBuf> {
    let directory = fs::symlink_metadata(path)?;
    if !directory.is_dir() || directory.file_type().is_symlink() {
        return Err(invalid("candidate directory is not ordinary"));
    }
    if expected_product.source_root() != root {
        return Err(invalid("wrong product source"));
    }
    let report_path = path.join(REPORT);
    let encoded = read_bounded(&report_path, MAX_REPORT_BYTES)?;
    if hex(Sha256::digest(&encoded)) != expected_report_sha256 {
        return Err(invalid("candidate report differs from selected digest"));
    }
    let report: serde_json::Value = serde_json::from_slice(&encoded).map_err(invalid_json)?;
    if report.as_object().map(|object| object.len()) != Some(9)
        || serde_json::to_vec(&report).map_err(invalid_json)? != encoded
        || report["format"] != 1
        || report["kind"] != "local-native-candidate"
        || report["buildReady"] != false
        || report["target"] != "headless-macos-arm64-dev-v1"
        || report["sourceGeneration"] != root.generation
        || report["sourceRevision"] != hex(root.revision.as_bytes())
        || report["productRevision"] != hex(expected_product.revision().as_bytes())
    {
        return Err(invalid("candidate report identity mismatch"));
    }
    let files = report["files"]
        .as_object()
        .ok_or_else(|| invalid("missing file index"))?;
    if files.len() < EVIDENCE.len() + 3
        || files.len() > MAX_FILES
        || !files.contains_key(SCENE)
        || !files.contains_key(BINARY)
        || EVIDENCE.iter().any(|name| !files.contains_key(*name))
    {
        return Err(invalid("incomplete candidate file index"));
    }
    let mut expected_names = files.keys().cloned().collect::<Vec<_>>();
    expected_names.push(REPORT.to_owned());
    expected_names.sort_unstable();
    let mut actual_names = Vec::with_capacity(expected_names.len());
    for entry in fs::read_dir(path)? {
        if actual_names.len() == expected_names.len() {
            return Err(invalid("candidate directory exceeds report membership"));
        }
        actual_names.push(
            entry?
                .file_name()
                .into_string()
                .map_err(|_| invalid("non-UTF8 candidate name"))?,
        );
    }
    actual_names.sort_unstable();
    if actual_names != expected_names {
        return Err(invalid("candidate file membership changed"));
    }
    let mut receipts = 0;
    for (name, expected) in files {
        if !safe_name(name) || name == REPORT {
            return Err(invalid("unsafe candidate name"));
        }
        if name.ends_with(".receipt") {
            receipts += 1;
        }
        let (length, hash) = stamp(&path.join(name), MAX_FILE_BYTES)?;
        if expected.as_object().map(|o| o.len()) != Some(2)
            || expected["bytes"] != length
            || expected["sha256"] != hash
        {
            return Err(invalid("candidate file differs from report"));
        }
    }
    if report["receiptCount"] != receipts
        || receipts == 0
        || files.len() != EVIDENCE.len() + 2 + receipts
    {
        return Err(invalid("receipt set changed"));
    }
    let environment = read_small_json(&path.join("environment.json"))?;
    if environment["sourceRevision"] != report["sourceRevision"]
        || environment["productRevision"] != report["productRevision"]
        || environment["target"] != report["target"]
        || environment["cacheManifestSha256"] != files["private-cache-manifest.json"]["sha256"]
        || environment["receiptWrapperSha256"] != files["rustc-receipt-wrapper"]["sha256"]
    {
        return Err(invalid("environment selection differs from candidate"));
    }
    let request = read_small_json(&path.join("cargo-supervisor-request.json"))?;
    if request["cwd"] != environment["sourcePath"]
        || request["output"] != environment["outputPath"]
        || request["environment"]["CARGO_TARGET_DIR"] != environment["outputPath"]
    {
        return Err(invalid(
            "supervisor request differs from environment selection",
        ));
    }
    let declaration = read_small_json(&path.join("build-declaration.json"))?;
    if declaration["package"] != "i1-mobile-game"
        || declaration["entryScene"] != expected_product.scene_id()
    {
        return Err(invalid("build declaration differs from candidate scene"));
    }
    let receipt = read_small_json(&path.join("receipt-verification.json"))?;
    if receipt["buildReady"] != false
        || receipt["compiles"] != 5
        || receipt["probes"].as_u64().is_none_or(|count| count == 0)
    {
        return Err(invalid("receipt verification differs from candidate"));
    }
    let mut receipt_hashes = files
        .iter()
        .filter_map(|(name, value)| {
            name.ends_with(".receipt")
                .then(|| value["sha256"].as_str().map(str::to_owned))
                .flatten()
        })
        .collect::<Vec<_>>();
    receipt_hashes.sort_unstable();
    if receipt["receiptSetSha256"] != hex(Sha256::digest(receipt_hashes.join("\n").as_bytes())) {
        return Err(invalid(
            "retained raw receipts differ from verifier summary",
        ));
    }
    let scene_bytes = read_bounded(&path.join(SCENE), crate::MAX_RECORD_BYTES as u64 + 1024)?;
    load_scene_artifact(
        &scene_bytes,
        expected_product.artifact().content_hash(),
        root,
        expected_product.revision(),
    )
    .map_err(|_| invalid("scene product differs from source/product key"))?;
    Ok(path.join(BINARY))
}

fn invalid_json(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant},
    };

    struct ChildGuard(Child);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    fn test_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "thrum-candidate-publish-{}-{}",
            std::process::id(),
            NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn failed_stage_and_existing_destination_preserve_prior_candidate() {
        let root = test_root();
        let destination = root.join("candidate");
        let failed = publish_directory(&destination, |stage| -> io::Result<()> {
            fs::write(stage.join("partial"), b"incomplete")?;
            Err(invalid("fixture build failed"))
        });
        assert!(failed.is_err());
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);

        publish_directory(&destination, |stage| {
            let mut file = fs::File::create(stage.join("complete"))?;
            file.write_all(b"old candidate")?;
            file.sync_all()
        })
        .unwrap();
        assert_eq!(
            fs::read(destination.join("complete")).unwrap(),
            b"old candidate"
        );
        let replacement = publish_directory(&destination, |_stage| -> io::Result<()> {
            panic!("existing destination must refuse before staging")
        });
        assert_eq!(
            replacement.unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read(destination.join("complete")).unwrap(),
            b"old candidate"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn process_kill_during_stage_does_not_expose_candidate() {
        let root = test_root();
        let destination = root.join("candidate");
        let marker = root.join("entered");
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "candidate_output::tests::staged_publish_child",
                "--ignored",
                "--nocapture",
            ])
            .env("THRUM_CANDIDATE_PUBLISH_CHILD_ROOT", &root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut child = ChildGuard(child);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.exists() && Instant::now() < deadline {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "stage child exited early"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert!(marker.exists(), "stage child did not enter build callback");
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        assert!(!destination.exists());
        assert!(fs::read_dir(&root).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".thrum-candidate-stage-")
        }));

        publish_directory(&destination, |stage| {
            let mut file = fs::File::create(stage.join("complete"))?;
            file.write_all(b"surviving candidate")?;
            file.sync_all()
        })
        .unwrap();
        assert_eq!(
            fs::read(destination.join("complete")).unwrap(),
            b"surviving candidate"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "spawned by candidate publication interruption test"]
    fn staged_publish_child() {
        let Some(root) = std::env::var_os("THRUM_CANDIDATE_PUBLISH_CHILD_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        publish_directory(&root.join("candidate"), |stage| {
            let mut file = fs::File::create(stage.join("partial"))?;
            file.write_all(b"incomplete")?;
            file.sync_all()?;
            fs::write(root.join("entered"), b"ready")?;
            thread::sleep(Duration::from_secs(30));
            Ok(())
        })
        .unwrap();
    }
}
