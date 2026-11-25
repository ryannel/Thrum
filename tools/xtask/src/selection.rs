//! The merge selection of `./dev check` (GATE-TIME mechanism 4): the workspace packages whose test
//! binaries a change can reach, by Cargo's own graph, the path rules and the declared edges.
//!
//! The change is every path that differs between the checkout and its merge base with the main
//! branch, uncommitted and untracked paths included. A path in a package selects that package and
//! every package that depends on it, directly or through others, as a normal, build or dev
//! dependency. A declared edge adds the one package whose tests read a path at run time. Where the
//! selection cannot be sure, it runs the whole workspace and says why.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
    process::Command,
};

/// The main branch a checkout's change is measured from: ORCHESTRATE.md's `Remote` and `Branch`.
pub(crate) const MAIN_BRANCH: &str = "origin/main";

/// Inputs every package's build or tests share. A change to one runs the whole workspace's tests.
/// A name that ends in `/` is a folder.
const SHARED_INPUTS: &[(&str, &str)] = &[
    ("Cargo.toml", "the root manifest"),
    ("Cargo.lock", "the lockfile"),
    ("rust-toolchain.toml", "the toolchain file"),
    ("clippy.toml", "the lint configuration"),
    (".cargo/", "Cargo's configuration"),
    ("dev", "`./dev` itself"),
    ("dev.ps1", "`./dev` itself"),
    ("tools/xtask/", "`tools/xtask`, the check itself"),
    (".config/long-tests.txt", "the test runner's configuration"),
    (".config/nextest.toml", "the test runner's configuration"),
];

/// The tracked evidence folder, which the merge refuses new files in. Spelt in two parts, as the
/// shape rule `evidence-path` spells it, because source never names the folder outright.
pub(crate) const TRACKED_EVIDENCE: &str = concat!("docs/", "evidence/");

/// Paths that a rule below would place but that select the whole workspace.
const WHOLE_FOLDERS: &[(&str, &str)] = &[(TRACKED_EVIDENCE, "the tracked evidence folder")];

/// Folders no Rust test reads: the guards and script suites that read them run on every merge.
/// Markdown anywhere is the same.
const NO_TEST_FOLDERS: &[&str] = &["docs/", ".agents/", ".github/", "scripts/", ".config/"];

/// A test that reads another package's files when it runs, which Cargo's graph cannot show: a
/// change under `path` selects `package`'s own test binaries as well, and nothing that depends on
/// it, because only the reader reads the path. A module that adds such a reader adds its edge here
/// in the same landing.
pub(crate) struct Edge {
    pub(crate) path: &'static str,
    pub(crate) package: &'static str,
    pub(crate) reader: &'static str,
}

pub(crate) const DECLARED_EDGES: &[Edge] = &[
    Edge {
        path: "crates/thrum-asset-gpu/src/",
        package: "thrum-render",
        reader: "every_cooked_pipeline_goes_through_the_seam_and_escapes_nothing scans that folder",
    },
    Edge {
        path: "tests/fixtures/game-ui/font-source/",
        package: "thrum-cook",
        reader: "font_cook and font_hostile read the original V1 font sources",
    },
    Edge {
        path: "tests/fixtures/game-ui/font-source/",
        package: "thrum-ui",
        reader: "text_runs lays out the cooked V1 font sources",
    },
    Edge {
        path: "tests/fixtures/game-ui/font-source/",
        package: "v1-game",
        reader: "interface_text_gpu reads the font sources",
    },
];

/// One workspace package: its folder relative to the workspace root, and the workspace packages it
/// depends on, as any kind of dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) folder: String,
    pub(crate) dependencies: Vec<String>,
}

/// The workspace's packages, from `cargo metadata --no-deps`.
#[derive(Debug, Clone, Default)]
pub(crate) struct Graph {
    pub(crate) packages: Vec<Package>,
}

fn json_str<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("cargo metadata gave no {key}"))
}

/// A folder with `/` separators and no trailing one.
fn folder_text(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned()
}

impl Graph {
    /// Reads the output of `cargo metadata --format-version 1 --no-deps`. Every declared
    /// dependency is kept, of every kind and for every target, so the graph never misses an edge.
    pub(crate) fn from_metadata(text: &str) -> Result<Self, String> {
        let metadata: serde_json::Value =
            serde_json::from_str(text).map_err(|error| format!("cargo metadata: {error}"))?;
        let root = Path::new(json_str(&metadata, "workspace_root")?);
        let listed = metadata
            .get("packages")
            .and_then(serde_json::Value::as_array)
            .ok_or("cargo metadata gave no packages")?;
        let mut folders = BTreeMap::new();
        for package in listed {
            let manifest = Path::new(json_str(package, "manifest_path")?);
            let folder = manifest.parent().ok_or("a manifest with no folder")?;
            folders.insert(folder_text(folder), json_str(package, "name")?.to_owned());
        }
        let mut packages = Vec::new();
        for package in listed {
            let manifest = Path::new(json_str(package, "manifest_path")?);
            let folder = manifest.parent().ok_or("a manifest with no folder")?;
            let relative = folder.strip_prefix(root).map_err(|_| {
                format!(
                    "package folder {} is outside the workspace root",
                    folder.display()
                )
            })?;
            let mut dependencies = BTreeSet::new();
            for dependency in package
                .get("dependencies")
                .and_then(serde_json::Value::as_array)
                .ok_or("cargo metadata gave no dependencies")?
            {
                let Some(path) = dependency.get("path").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let path = folder_text(&normalise(Path::new(path)));
                if let Some(name) = folders.get(&path) {
                    dependencies.insert(name.clone());
                }
            }
            packages.push(Package {
                name: json_str(package, "name")?.to_owned(),
                folder: folder_text(relative),
                dependencies: dependencies.into_iter().collect(),
            });
        }
        Ok(Self { packages })
    }

    /// Runs `cargo metadata` in the checkout.
    pub(crate) fn read() -> Result<Self, String> {
        let output = Command::new(env!("CARGO"))
            .args([
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--locked",
                "--offline",
            ])
            .output()
            .map_err(|error| format!("cargo metadata did not start: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Self::from_metadata(&String::from_utf8_lossy(&output.stdout))
    }

    /// The package whose folder holds `path`: the deepest, if folders nest.
    fn owner(&self, path: &str) -> Option<&Package> {
        self.packages
            .iter()
            .filter(|package| under(path, &package.folder))
            .max_by_key(|package| package.folder.len())
    }
}

/// Removes `.` and `..` from a path without touching the file system.
fn normalise(path: &Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Whether `path` is `folder` or lies under it.
fn under(path: &str, folder: &str) -> bool {
    path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether `path` matches a rule's name, where a name that ends in `/` is a folder.
fn matches(path: &str, rule: &str) -> bool {
    match rule.strip_suffix('/') {
        Some(folder) => under(path, folder),
        None => path == rule,
    }
}

/// What one changed path selects by the path rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Rule {
    /// The whole workspace, for the reason given.
    Whole(String),
    /// No Rust test.
    NoTest(&'static str),
    /// This package and every package that depends on it.
    Package(String),
}

/// The path rules, in order: a shared input or the tracked evidence folder runs everything; Markdown and the
/// folders no Rust test reads run nothing; a package's folder selects the package; anything else
/// runs everything.
pub(crate) fn rule(graph: &Graph, path: &str) -> Rule {
    if let Some((_, what)) = SHARED_INPUTS.iter().find(|(name, _)| matches(path, name)) {
        return Rule::Whole(format!("{path} is {what}, a shared input"));
    }
    if let Some((_, what)) = WHOLE_FOLDERS.iter().find(|(name, _)| matches(path, name)) {
        return Rule::Whole(format!("{path} is under {what}, which selects everything"));
    }
    if Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return Rule::NoTest("Markdown");
    }
    if let Some(package) = graph.owner(path) {
        return Rule::Package(package.name.clone());
    }
    if let Some(folder) = NO_TEST_FOLDERS.iter().find(|name| matches(path, name)) {
        return Rule::NoTest(folder);
    }
    Rule::Whole(format!(
        "{path} has no path rule (a new folder outside a package adds one)"
    ))
}

/// What `./dev check` runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Selection {
    /// Every test binary of the workspace, and why.
    Whole(String),
    /// The test binaries of these packages, each with why it was selected; empty runs no Rust test.
    Packages(BTreeMap<String, String>),
}

/// Selects from the changed paths: the packages that hold one, every package that depends on one
/// of those, and the package of each declared edge a path lies under. One path the rules send to
/// the whole workspace runs everything.
pub(crate) fn select(graph: &Graph, paths: &[String]) -> Selection {
    let mut seeds: BTreeMap<String, String> = BTreeMap::new();
    for path in paths {
        match rule(graph, path) {
            Rule::Whole(why) => return Selection::Whole(why),
            Rule::NoTest(_) => {}
            Rule::Package(name) => {
                seeds
                    .entry(name)
                    .or_insert_with(|| format!("holds the changed path {path}"));
            }
        }
    }
    // Breadth first from the packages that hold a change, so each reason names the shortest chain.
    let mut chosen = seeds.clone();
    let mut queue: VecDeque<String> = seeds.into_keys().collect();
    while let Some(name) = queue.pop_front() {
        for dependant in &graph.packages {
            if dependant.dependencies.contains(&name) && !chosen.contains_key(&dependant.name) {
                chosen.insert(dependant.name.clone(), format!("depends on {name}"));
                queue.push_back(dependant.name.clone());
            }
        }
    }
    for edge in DECLARED_EDGES {
        if let Some(path) = paths.iter().find(|path| matches(path, edge.path)) {
            chosen.entry(edge.package.to_owned()).or_insert_with(|| {
                format!(
                    "a declared edge: {path} is under {}, and {}",
                    edge.path, edge.reader
                )
            });
        }
    }
    Selection::Packages(chosen)
}

fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| format!("git did not start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn nul_paths(bytes: &[u8]) -> impl Iterator<Item = String> + '_ {
    bytes
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).into_owned())
}

/// The change: the merge base with `main`, and every path that differs between the checkout and
/// it, uncommitted and untracked paths included. A renamed file counts at both its paths. An
/// error says why the change cannot be computed, and the check then runs the whole workspace.
pub(crate) fn change(dir: &Path, main: &str) -> Result<(String, Vec<String>), String> {
    let base = git(dir, &["merge-base", "HEAD", main])
        .map_err(|error| format!("this checkout has no merge base with {main} ({error})"))?;
    let base = String::from_utf8_lossy(&base).trim().to_owned();
    let mut paths: BTreeSet<String> = nul_paths(&git(
        dir,
        &["diff", "--name-only", "--no-renames", "-z", &base, "--"],
    )?)
    .collect();
    paths.extend(nul_paths(&git(
        dir,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?));
    if paths.is_empty() {
        return Err(format!(
            "this checkout does not differ from {main}, as on main itself"
        ));
    }
    Ok((base, paths.into_iter().collect()))
}

/// The selection for the checkout in `dir`, with the lines that say what it selected and why.
pub(crate) fn for_checkout(dir: &Path) -> (Selection, Vec<String>) {
    let (base, paths) = match change(dir, MAIN_BRANCH) {
        Ok(change) => change,
        Err(why) => {
            return (
                Selection::Whole(why.clone()),
                report(&Selection::Whole(why), None),
            );
        }
    };
    let graph = match Graph::read() {
        Ok(graph) => graph,
        Err(error) => {
            let selection =
                Selection::Whole(format!("the dependency graph cannot be read: {error}"));
            let lines = report(&selection, None);
            return (selection, lines);
        }
    };
    let selection = select(&graph, &paths);
    let whence = format!(
        "differ from the merge base {} with {MAIN_BRANCH}",
        base.get(..12).unwrap_or(&base)
    );
    let lines = report(&selection, Some((&graph, &whence, &paths)));
    (selection, lines)
}

/// How many changed paths the report lists by name.
const LISTED_PATHS: usize = 12;

/// What the check prints about its selection; `change` is the graph, where the paths came from,
/// and the paths.
pub(crate) fn report(
    selection: &Selection,
    change: Option<(&Graph, &str, &[String])>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some((graph, whence, paths)) = change {
        lines.push(format!("the change: {} paths {whence}", paths.len()));
        for path in paths.iter().take(LISTED_PATHS) {
            let what = match rule(graph, path) {
                Rule::Whole(_) => "the whole workspace".to_owned(),
                Rule::NoTest(why) => format!("no Rust test ({why})"),
                Rule::Package(name) => format!("package {name}"),
            };
            lines.push(format!("  {path}: {what}"));
        }
        if paths.len() > LISTED_PATHS {
            lines.push(format!("  and {} more", paths.len() - LISTED_PATHS));
        }
    }
    match selection {
        Selection::Whole(why) => {
            lines.push(format!("selected: the whole workspace, because {why}"));
        }
        Selection::Packages(chosen) if chosen.is_empty() => {
            lines.push("selected: no Rust test binary; the change reaches no package".to_owned());
        }
        Selection::Packages(chosen) => {
            lines.push(format!(
                "selected: the test binaries of {} packages",
                chosen.len()
            ));
            for (name, why) in chosen {
                lines.push(format!("  {name}: {why}"));
            }
        }
    }
    lines
}

/// The cargo-nextest filter for a selection of packages, or `None` for the whole workspace.
pub(crate) fn nextest_filter(selection: &Selection) -> Option<String> {
    match selection {
        Selection::Whole(_) => None,
        Selection::Packages(chosen) if chosen.is_empty() => Some("none()".to_owned()),
        Selection::Packages(chosen) => Some(
            chosen
                .keys()
                .map(|name| format!("package(={name})"))
                .collect::<Vec<_>>()
                .join(" | "),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DECLARED_EDGES, Graph, MAIN_BRANCH, Package, Rule, Selection, TRACKED_EVIDENCE, change,
        nextest_filter, rule, select,
    };
    use std::path::Path;

    /// A small workspace shaped like Thrum's: `core` at the bottom, `render` on it, `asset-gpu`
    /// on `render`, a game on everything, a model nothing depends on, and edges of each kind.
    fn graph() -> Graph {
        let package = |name: &str, folder: &str, dependencies: &[&str]| Package {
            name: name.into(),
            folder: folder.into(),
            dependencies: dependencies.iter().map(|d| (*d).into()).collect(),
        };
        Graph {
            packages: vec![
                package("thrum-core", "crates/thrum-core", &[]),
                package("thrum-render", "crates/thrum-render", &["thrum-core"]),
                package(
                    "thrum-asset-gpu",
                    "crates/thrum-asset-gpu",
                    &["thrum-render"],
                ),
                package("thrum-cook", "crates/thrum-cook", &["thrum-asset-gpu"]),
                package("thrum-rain-model", "crates/thrum-rain-model", &[]),
                package("thrum-test-support", "crates/thrum-test-support", &[]),
                package("thrum-sea", "crates/thrum-sea", &["thrum-test-support"]),
                package(
                    "v1-game",
                    "examples/v1-game",
                    &["thrum-cook", "thrum-render"],
                ),
                package("v1-game-ios", "examples/v1-game-ios", &["v1-game"]),
                package("xtask", "tools/xtask", &[]),
            ],
        }
    }

    fn picked(paths: &[&str]) -> Selection {
        select(
            &graph(),
            &paths.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>(),
        )
    }

    fn names(selection: &Selection) -> Vec<&str> {
        match selection {
            Selection::Packages(chosen) => chosen.keys().map(String::as_str).collect(),
            Selection::Whole(why) => panic!("expected packages, got the whole workspace: {why}"),
        }
    }

    fn whole(selection: &Selection) -> &str {
        match selection {
            Selection::Whole(why) => why,
            Selection::Packages(chosen) => panic!("expected the whole workspace, got {chosen:?}"),
        }
    }

    #[test]
    fn selection_of_a_package_nothing_depends_on_is_that_package() {
        let selection = picked(&["crates/thrum-rain-model/src/lib.rs"]);
        assert_eq!(names(&selection), ["thrum-rain-model"]);
        assert_eq!(
            nextest_filter(&selection).as_deref(),
            Some("package(=thrum-rain-model)")
        );
    }

    #[test]
    fn selection_of_a_package_others_depend_on_takes_every_dependant() {
        let selection = picked(&["crates/thrum-core/src/lib.rs"]);
        assert_eq!(
            names(&selection),
            [
                "thrum-asset-gpu",
                "thrum-cook",
                "thrum-core",
                "thrum-render",
                "v1-game",
                "v1-game-ios"
            ]
        );
        let Selection::Packages(chosen) = selection else {
            unreachable!()
        };
        assert_eq!(
            chosen["thrum-core"],
            "holds the changed path crates/thrum-core/src/lib.rs"
        );
        assert_eq!(chosen["thrum-render"], "depends on thrum-core");
        // The shortest chain: the game depends on render directly.
        assert_eq!(chosen["v1-game"], "depends on thrum-render");
        assert_eq!(chosen["v1-game-ios"], "depends on v1-game");
    }

    #[test]
    fn selection_follows_a_dev_and_a_build_dependency_from_cargo_metadata() {
        let metadata = r#"{"workspace_root": "/w", "packages": [
            {"name": "support", "manifest_path": "/w/crates/support/Cargo.toml", "dependencies": []},
            {"name": "gen", "manifest_path": "/w/crates/gen/Cargo.toml", "dependencies": []},
            {"name": "sea", "manifest_path": "/w/crates/sea/Cargo.toml", "dependencies": [
                {"name": "support", "kind": "dev", "path": "/w/crates/support"},
                {"name": "generator", "rename": "gen", "kind": "build", "path": "/w/crates/sea/../gen"},
                {"name": "serde", "kind": null}
            ]},
            {"name": "game", "manifest_path": "/w/examples/game/Cargo.toml", "dependencies": [
                {"name": "sea", "kind": null, "target": "cfg(target_os = \"macos\")", "path": "/w/crates/sea"}
            ]}
        ]}"#;
        let graph = Graph::from_metadata(metadata).unwrap();
        let sea = &graph.packages[2];
        assert_eq!(sea.folder, "crates/sea");
        assert_eq!(sea.dependencies, ["gen", "support"]);
        let paths = |path: &str| vec![path.to_owned()];
        // Only a dev dependency joins sea to support, so a change to support still selects sea.
        assert_eq!(
            names(&select(&graph, &paths("crates/support/src/lib.rs"))),
            ["game", "sea", "support"]
        );
        assert_eq!(
            names(&select(&graph, &paths("crates/gen/src/lib.rs"))),
            ["game", "gen", "sea"]
        );
    }

    #[test]
    fn selection_takes_a_dev_dependency_edge_in_the_graph() {
        // thrum-sea uses thrum-test-support only in its tests.
        assert_eq!(
            names(&picked(&["crates/thrum-test-support/src/lib.rs"])),
            ["thrum-sea", "thrum-test-support"]
        );
    }

    #[test]
    fn selection_of_a_declared_edge_adds_the_reader_and_none_of_its_dependants() {
        let selection = picked(&["crates/thrum-asset-gpu/src/staging.rs"]);
        // render by the edge; asset-gpu, cook and the game by the graph. Nothing that depends on
        // render alone joins through the edge.
        assert_eq!(
            names(&selection),
            [
                "thrum-asset-gpu",
                "thrum-cook",
                "thrum-render",
                "v1-game",
                "v1-game-ios"
            ]
        );
        let Selection::Packages(chosen) = &selection else {
            unreachable!()
        };
        assert!(
            chosen["thrum-render"]
                .starts_with("a declared edge: crates/thrum-asset-gpu/src/staging.rs"),
            "{}",
            chosen["thrum-render"]
        );
        // A file of asset-gpu outside the scanned folder is not under the edge.
        assert!(
            !names(&picked(&["crates/thrum-asset-gpu/tests/mapped_indices.rs"]))
                .contains(&"thrum-render")
        );
        // A package that depends on render and not on asset-gpu is not selected by the edge.
        let mut graph = graph();
        graph.packages.push(Package {
            name: "thrum-clouds".into(),
            folder: "crates/thrum-clouds".into(),
            dependencies: vec!["thrum-render".into()],
        });
        let selection = select(&graph, &["crates/thrum-asset-gpu/src/lib.rs".to_owned()]);
        assert!(!names(&selection).contains(&"thrum-clouds"));
        // Each edge names a folder of the package it reads and a package other than that one.
        for edge in DECLARED_EDGES {
            assert!(edge.path.ends_with('/'), "{}", edge.path);
            let owner = rule(&graph, edge.path.trim_end_matches('/'));
            assert_ne!(owner, Rule::Package(edge.package.into()), "{}", edge.path);
        }
    }

    #[test]
    fn selection_runs_the_whole_workspace_for_each_shared_input() {
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "clippy.toml",
            ".cargo/config.toml",
            "dev",
            "dev.ps1",
            "tools/xtask/src/main.rs",
            "tools/xtask/Cargo.toml",
            ".config/long-tests.txt",
            ".config/nextest.toml",
        ] {
            let why = whole(&picked(&["crates/thrum-rain-model/src/lib.rs", path])).to_owned();
            assert!(
                why.starts_with(path) && why.contains("a shared input"),
                "{path}: {why}"
            );
        }
        // A package's own manifest is the package's, not the root manifest.
        assert_eq!(
            names(&picked(&["crates/thrum-rain-model/Cargo.toml"])),
            ["thrum-rain-model"]
        );
    }

    #[test]
    fn selection_runs_the_whole_workspace_for_a_path_with_no_rule() {
        let record = format!("{TRACKED_EVIDENCE}WATER/M2/record.json");
        for path in [
            "tests/fixtures/tripwires/cooked-edit.tsv",
            record.as_str(),
            "newfolder/thing.rs",
            ".gitignore",
            "crates/orphan/src/lib.rs",
        ] {
            assert!(!whole(&picked(&[path])).is_empty(), "{path}");
        }
        assert!(whole(&picked(&["newfolder/thing.rs"])).contains("no path rule"));
        let evidence = |name: &str| format!("{TRACKED_EVIDENCE}{name}");
        assert!(whole(&picked(&[&evidence("a.png")])).contains("tracked evidence"));
        // Markdown in the tracked evidence folder still selects everything.
        assert!(whole(&picked(&[&evidence("WATER/M2.md")])).contains("tracked evidence"));
    }

    #[test]
    fn selection_takes_no_rust_test_for_documents_scripts_and_guard_data() {
        for path in [
            "README.md",
            "crates/thrum-core/README.md",
            "docs/plans/GATE-TIME/DONE.json",
            "docs/design/maturity.json",
            ".agents/skills/thrum-engineering/SKILL.md",
            ".github/workflows/ci.yml",
            "scripts/repo_check.py",
            ".config/architecture.toml",
            ".config/sim2d-profile/profile.json",
        ] {
            assert_eq!(
                picked(&[path]),
                Selection::Packages(std::collections::BTreeMap::default()),
                "{path}"
            );
        }
        let empty = picked(&["scripts/repo_check.py", "AGENTS.md"]);
        assert_eq!(nextest_filter(&empty).as_deref(), Some("none()"));
    }

    #[test]
    fn selection_places_a_path_in_the_package_whose_folder_holds_it() {
        let graph = graph();
        // examples/v1-game is a prefix of examples/v1-game-ios's folder text, not its folder.
        assert_eq!(
            rule(&graph, "examples/v1-game-ios/src/lib.rs"),
            Rule::Package("v1-game-ios".into())
        );
        assert_eq!(
            rule(&graph, "examples/v1-game/src/lib.rs"),
            Rule::Package("v1-game".into())
        );
        assert!(matches!(
            rule(&graph, "examples/v1-gamex/a.rs"),
            Rule::Whole(_)
        ));
    }

    #[test]
    fn selection_of_a_change_holds_uncommitted_and_untracked_paths() {
        let dir = std::env::temp_dir().join(format!("xtask-selection-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(dir.join("crates/a/src")).unwrap();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args([
                    "-c",
                    "commit.gpgsign=false",
                    "-c",
                    "core.hooksPath=/dev/null",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(status.status.success(), "git {args:?}: {status:?}");
        };
        git(&["init", "--quiet", "--initial-branch=work"]);
        // No main branch yet: no merge base.
        std::fs::write(dir.join("crates/a/src/lib.rs"), "a").unwrap();
        std::fs::write(dir.join("crates/a/src/old.rs"), "old").unwrap();
        git(&["add", "."]);
        git(&["commit", "--quiet", "-m", "start"]);
        let no_base = change(&dir, MAIN_BRANCH).unwrap_err();
        assert!(no_base.contains("no merge base"), "{no_base}");
        git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        let same = change(&dir, MAIN_BRANCH).unwrap_err();
        assert!(same.contains("does not differ"), "{same}");
        // A commit, a rename, an uncommitted edit, a staged file and an untracked one all count.
        std::fs::write(dir.join("committed.rs"), "c").unwrap();
        git(&["add", "."]);
        git(&["commit", "--quiet", "-m", "c"]);
        git(&["mv", "crates/a/src/old.rs", "crates/a/src/new.rs"]);
        std::fs::write(dir.join("staged.rs"), "s").unwrap();
        git(&["add", "staged.rs"]);
        std::fs::write(dir.join("crates/a/src/lib.rs"), "changed").unwrap();
        std::fs::write(dir.join("untracked.rs"), "u").unwrap();
        let (base, paths) = change(&dir, MAIN_BRANCH).unwrap();
        assert_eq!(base.len(), 40);
        assert_eq!(
            paths,
            [
                "committed.rs",
                "crates/a/src/lib.rs",
                "crates/a/src/new.rs",
                "crates/a/src/old.rs",
                "staged.rs",
                "untracked.rs"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(!Path::new(&dir).exists());
    }
}
