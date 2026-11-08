//! G3t D0.3 coverage check: every production pipeline in this crate is created
//! through a [`DeviceSite`](crate::runtime::DeviceSite), the raw-device escapes
//! that bypass a generation's ledger are an exact inventory, and the cooked
//! asset crate holds no raw site or escape either. It runs in default builds.

use std::path::{Path, PathBuf};

const RAW: [&str; 2] = ["create_render_pipeline(", "create_compute_pipeline("];
/// Ways to build a site that reaches no generation's ledger.
const ESCAPES: [&str; 2] = ["DeviceSite::untracked(", "capture_device("];
/// The seam itself, the only production files that may call wgpu directly.
const SEAM: [&str; 2] = ["runtime/creation.rs", "runtime/creation_off.rs"];
/// The production escapes: `capture_device` wraps an untracked site, and the
/// two public raw-device `PreparedSynthetic` entry points re-capture with it.
const ESCAPE_INVENTORY: [(&str, usize); 2] = [("capabilities.rs", 1), ("synthetic.rs", 2)];

fn is_test_path(path: &Path) -> bool {
    path.components().any(|part| part.as_os_str() == "tests")
        || path.file_name().is_some_and(|name| name == "tests.rs")
}

fn sources(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root).expect("read source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Code lines of `source` containing one of `needles`, skipping comments and
/// the escapes' own definitions.
fn sites(source: &str, needles: &[&str]) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.trim_start();
            !code.starts_with("//") && !code.starts_with("pub fn ") && !code.starts_with("fn ")
        })
        .filter(|(_, line)| needles.iter().any(|needle| line.contains(needle)))
        .map(|(index, line)| (index + 1, line.trim().to_owned()))
        .collect()
}

/// `(relative, "relative:line: text")` for each site in the production
/// sources under `root`.
fn production_sites(root: &Path, skip: &[&str], needles: &[&str]) -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(root, &mut files);
    files.sort();
    let mut found = Vec::new();
    for file in files {
        let relative = file.strip_prefix(root).expect("under root");
        if is_test_path(relative) {
            continue;
        }
        let relative = relative.to_string_lossy().replace('\\', "/");
        if skip.contains(&relative.as_str()) {
            continue;
        }
        let source = std::fs::read_to_string(&file).expect("read source");
        for (line, text) in sites(&source, needles) {
            found.push((relative.clone(), format!("{relative}:{line}: {text}")));
        }
    }
    found
}

fn listing(sites: &[(String, String)]) -> String {
    sites
        .iter()
        .map(|(_, site)| site.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_scanner_flags_unwrapped_sites_and_escapes_and_ignores_the_seam_call() {
    let unwrapped = "let p = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {";
    let chained = "        .create_compute_pipeline(&desc)";
    let wrapped = "let p = device.render(&wgpu::RenderPipelineDescriptor {";
    let comment = "// device.create_render_pipeline(&desc) is wrapped below";
    assert_eq!(sites(unwrapped, &RAW).len(), 1);
    assert_eq!(sites(chained, &RAW).len(), 1);
    assert_eq!(sites(wrapped, &RAW), []);
    assert_eq!(sites(comment, &RAW), []);
    let escape = "let site = crate::runtime::DeviceSite::untracked(device.clone());";
    let recapture = "let caps = DeviceCapabilities::capture_device(device, 7);";
    let definition = "pub fn capture_device(device: &wgpu::Device, generation: u64) -> Self {";
    assert_eq!(sites(escape, &ESCAPES).len(), 1);
    assert_eq!(sites(recapture, &ESCAPES).len(), 1);
    assert_eq!(sites(definition, &ESCAPES), []);
}

#[test]
fn every_production_pipeline_in_the_renderer_goes_through_the_seam() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let raw = production_sites(&root, &SEAM, &RAW);
    assert!(
        raw.is_empty(),
        "create these through the generation's DeviceSite (`device.render` / `.compute`):\n{}",
        listing(&raw)
    );
}

#[test]
fn production_raw_device_escapes_are_the_listed_ones() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let escapes = production_sites(&root, &SEAM, &ESCAPES);
    for (file, count) in ESCAPE_INVENTORY {
        let found = escapes
            .iter()
            .filter(|(relative, _)| relative == file)
            .count();
        assert_eq!(found, count, "escapes in {file}:\n{}", listing(&escapes));
    }
    let listed: usize = ESCAPE_INVENTORY.iter().map(|(_, count)| count).sum();
    assert_eq!(
        escapes.len(),
        listed,
        "a production site escapes its generation's ledger:\n{}",
        listing(&escapes)
    );
}

#[test]
fn every_cooked_pipeline_goes_through_the_seam_and_escapes_nothing() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../thrum-asset-gpu/src");
    let raw = production_sites(&root, &[], &RAW);
    assert!(
        raw.is_empty(),
        "create cooked pipelines through `SceneSurface::device`:\n{}",
        listing(&raw)
    );
    let escapes = production_sites(&root, &[], &ESCAPES);
    assert!(escapes.is_empty(), "{}", listing(&escapes));
}
