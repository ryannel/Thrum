"""Adversarial fixtures for checks that guard subsequent agent contributions."""
import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import design_hashes
import repo_check


class DevLauncher(unittest.TestCase):
    @unittest.skipUnless(shutil.which("bash"), "the Unix dev launcher requires Bash")
    def test_configured_wrapper_is_opt_in_and_explicit_wrapper_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            tools = Path(directory)
            rustup = tools / "rustup"
            rustup.write_text(
                f"#!{sys.executable}\n"
                "import json, os, sys\n"
                "if sys.argv[1:2] == ['which']:\n"
                f"    print({str(tools / 'rustc')!r})\n"
                "else:\n"
                "    print(json.dumps({'wrapper': os.environ.get('RUSTC_WRAPPER', 'configured-wrapper')}))\n"
            )
            rustup.chmod(0o755)
            for supplied in [None, "", "/explicit/compiler-wrapper"]:
                with self.subTest(supplied=supplied):
                    environment = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"])
                    environment.pop("RUSTC_WRAPPER", None)
                    if supplied is not None:
                        environment["RUSTC_WRAPPER"] = supplied
                    result = subprocess.run(
                        ["bash", str(repo_check.ROOT / "dev"), "help"],
                        env=environment, text=True, capture_output=True, check=True,
                    )
                    self.assertEqual(json.loads(result.stdout)["wrapper"], supplied or "")


class RepositoryChecks(unittest.TestCase):
    def test_markdown_walk_keeps_local_docs_but_excludes_nested_git_roots(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / ".git").mkdir()
            local = root / ".claude" / "notes.md"
            local.parent.mkdir()
            local.write_text("[broken local link](missing.md)\n")
            for name, worktree in (("worktrees/g5", True), ("clone", False)):
                nested = root / ".claude" / name
                nested.mkdir(parents=True)
                if worktree:
                    (nested / ".git").write_text("gitdir: /some/main/.git/worktrees/g5\n")
                else:
                    (nested / ".git").mkdir()
                (nested / "README.md").write_text("[unfinished](missing.md)\n")
            files = repo_check.markdown_files(root)
            self.assertEqual(files, [local])
            errors, count = repo_check.check_links(root.resolve(), files)
            self.assertEqual(count, 1)
            self.assertEqual(len(errors), 1)

    def test_links_reject_missing_file_anchor_and_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            page = root / "index.md"
            page.write_text('# Here\n[ok](#here)\n[no](missing.md)\n[bad](#absent)\n[out](../outside.md)\n```md\n[example](ignored.md)\n```\n')
            errors, count = repo_check.check_links(root, [page])
            self.assertEqual(count, 4)
            self.assertEqual(len(errors), 3)
            self.assertTrue(any("missing anchor" in e for e in errors))

    def test_rust_layout_rejects_test_items_in_production_and_bare_ignore(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            src = root / "crates/core/src/lib.rs"
            src.parent.mkdir(parents=True)
            src.write_text(
                "#[cfg(test)]\nmod tests;\n"
                "#[cfg(test)]\n#[path = \"x.rs\"]\npub(crate) mod observer {}\n"
                "#[cfg(test)]\nfn probe() {}\n"
                "#[test]\n#[ignore = \"device only\"]\nfn reasoned() {}\n"
                "#[test]\n#[ignore]\nfn bare() {}\n"
            )
            integration = root / "crates/core/tests/it.rs"
            integration.parent.mkdir()
            integration.write_text("#[cfg(test)]\nfn helper() {}\n")
            errors, notes = repo_check.check_rust_layout(root, repo_check.rust_files(root))
            self.assertEqual(len(errors), 2)
            self.assertEqual(notes, [])
            self.assertTrue(any("lib.rs:6" in e and "test module" in e for e in errors))
            self.assertTrue(any("lib.rs:12" in e and "needs a reason" in e for e in errors))

    def test_file_size_counts_code_only_and_separates_note_from_limit(self):
        note_at, limit = repo_check.FILE_LINES["production"]
        body = "fn f() {}\n"
        padding = "\n// comment\n" * limit
        inline_tests = "#[cfg(test)]\nmod tests {\n" + body * limit + "}\n"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            src = root / "crates/core/src"
            src.mkdir(parents=True)
            (src / "small.rs").write_text(body * note_at + padding + inline_tests)
            (src / "noted.rs").write_text(body * (note_at + 1))
            (src / "large.rs").write_text(body * (limit + 1))
            (src / "tests.rs").write_text(body * (limit + 1))
            errors, notes = repo_check.check_rust_layout(root, repo_check.rust_files(root))
            self.assertEqual([e.split(":")[0] for e in errors], ["crates/core/src/large.rs"])
            self.assertEqual([n.split(":")[0] for n in notes], ["crates/core/src/noted.rs", "crates/core/src/tests.rs"])

    def test_explicit_and_duplicate_heading_anchors(self):
        self.assertEqual(repo_check.anchors('# One\n# One\n<a id="manual"></a>\n'), {'one', 'one-1', 'manual'})

    def test_dependency_policy_catches_renamed_target_build_edge(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            packages = []
            policy = {"packages": {}}
            for name in ("core", "tools"):
                path = root / name / "Cargo.toml"
                path.parent.mkdir()
                path.write_text('[package]\nversion.workspace=true\nedition.workspace=true\nrust-version.workspace=true\npublish.workspace=true\n[lints]\nworkspace=true\n')
                packages.append({"id": name, "name": name, "manifest_path": str(path), "dependencies": []})
                policy["packages"][name] = {"manifest": f"{name}/Cargo.toml", "allowed_workspace_dependencies": []}
            metadata = {"workspace_members": ["core", "tools"], "packages": packages}
            self.assertEqual(repo_check.check_packages(root, metadata, policy), [])
            packages[0]["dependencies"] = [{"name": "tools", "rename": "innocent_alias", "path": str(root / "tools"), "kind": "build", "target": 'cfg(windows)'}]
            self.assertIn("forbidden workspace dependency: core -> tools", repo_check.check_packages(root, metadata, policy))
            policy["packages"]["core"]["allowed_workspace_dependencies"] = ["tools"]
            self.assertEqual(repo_check.check_packages(root, metadata, policy), [])
            del policy["packages"]["tools"]
            self.assertTrue(any("registry mismatch" in e for e in repo_check.check_packages(root, metadata, policy)))

    def test_unregistered_manifest_is_rejected_without_a_folder_count_limit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            registered = root / "crates" / "core" / "Cargo.toml"
            registered.parent.mkdir(parents=True)
            registered.write_text("[package]\nname='core'\n")
            metadata = {"workspace_members": ["core"], "packages": [
                {"id": "core", "name": "core", "manifest_path": str(registered)}]}
            self.assertEqual(repo_check.check_manifest_inventory(root, metadata), [])
            extra = root / "crates" / "new_module" / "Cargo.toml"
            extra.parent.mkdir()
            extra.write_text("[package]\nname='new_module'\n")
            self.assertEqual(repo_check.check_manifest_inventory(root, metadata),
                             ["unregistered package manifest: crates/new_module/Cargo.toml"])

    def test_long_test_must_sit_in_a_listed_binary_that_holds_only_long_tests(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            tests = root / "crates" / "core" / "tests"
            (tests / "support").mkdir(parents=True)
            soak = tests / "support" / "soak.rs"
            soak.write_text("mod fly;\n#[test]\nfn quick() {}\n")
            (tests / "support" / "fly.rs").write_text(
                "#[test]\n#[ignore = \"long: ./dev long-tests runs it\"]\nfn hour() {}\n")
            other = tests / "other.rs"
            other.write_text("#[test]\n#[ignore = \"needs a cooked store\"]\nfn cooked() {}\n")
            metadata = {"workspace_members": ["core"], "packages": [{
                "id": "core", "name": "core", "targets": [
                    {"kind": ["test"], "name": "soak", "src_path": str(soak)},
                    {"kind": ["test"], "name": "other", "src_path": str(other)}]}]}
            listed = "# comment\ncore soak release\n"
            check = repo_check.check_long_tests
            self.assertEqual(check(root, metadata, listed), [])
            unlisted = check(root, metadata, "")
            self.assertEqual(len(unlisted), 1)
            self.assertIn("crates/core/tests/support/fly.rs:2: long test hour", unlisted[0])
            self.assertIn(".config/long-tests.txt", unlisted[0])
            other.write_text("#[test]\n#[ignore = \"long: probe\"]\nfn probe() {}\n")
            self.assertTrue(any("long test probe" in e for e in check(root, metadata, listed)))
            soak.write_text("mod fly;\n#[test]\n#[ignore = \"measurement\"]\nfn timing() {}\n")
            other.write_text("")
            mixed = check(root, metadata, listed)
            self.assertEqual(len(mixed), 1)
            self.assertIn("ignored test timing is in core soak", mixed[0])
            malformed = check(root, metadata, "core soak fast\ncore gone debug\n")
            self.assertTrue(any(e.startswith(".config/long-tests.txt:1: expected `<package> <test binary> release|debug`") for e in malformed))
            self.assertIn(".config/long-tests.txt: core has no test binary gone", malformed)

    def test_test_file_no_target_compiles_fails_and_names_where_to_declare_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            tests = root / "crates" / "core" / "tests"
            (tests / "support").mkdir(parents=True)
            (tests / "lanes").mkdir()
            (tests / "fixtures").mkdir()
            it = tests / "it.rs"
            it.write_text("mod support;\nmod clock;\nmod nested {\n    mod deep;\n}\n")
            # A helper only a declared file reaches, and one only a helper's inline module reaches.
            (tests / "clock.rs").write_text("use crate::support;\nmod lanes {\n    mod lane;\n}\n#[test]\nfn ticks() {}\n")
            (tests / "clock").mkdir()
            (tests / "clock" / "lanes").mkdir()
            (tests / "clock" / "lanes" / "lane.rs").write_text("#[test]\nfn lane() {}\n")
            (tests / "support.rs").write_text("pub(crate) mod world;\n// mod commented_out;\n")
            (tests / "support" / "world.rs").write_text("pub(crate) fn world() {}\n")
            (tests / "nested").mkdir()
            (tests / "nested" / "deep.rs").write_text("#[test]\nfn deep() {}\n")
            # A bench whose files sit in `tests/` compiles them.
            (tests / "fixtures" / "bench.rs").write_text("fn main() {}\n")
            manifest = root / "crates" / "core" / "Cargo.toml"
            manifest.write_text("[package]\n")
            metadata = {"workspace_members": ["core"], "packages": [{
                "id": "core", "name": "core", "manifest_path": str(manifest), "targets": [
                    {"kind": ["lib"], "name": "core", "src_path": str(root / "crates/core/src/lib.rs")},
                    {"kind": ["test"], "name": "it", "src_path": str(it)},
                    {"kind": ["bench"], "name": "bench", "src_path": str(tests / "fixtures" / "bench.rs")}]}]}
            check = repo_check.check_test_files_compiled
            self.assertEqual(check(root, metadata), [])
            # A file left out of `it`, one only a comment names, and one in a folder nothing declares.
            (tests / "drift.rs").write_text("#[test]\nfn drifts() {}\n")
            (tests / "support" / "commented_out.rs").write_text("")
            (tests / "lanes" / "lane.rs").write_text("#[test]\nfn lane() {}\n")
            errors = check(root, metadata)
            self.assertEqual(len(errors), 3, errors)
            self.assertTrue(errors[0].startswith("crates/core/tests/drift.rs: no test target of core compiles this file"))
            self.assertIn("declare it as `mod drift;` in crates/core/tests/it.rs", errors[0])
            self.assertIn("as a [[test]] in crates/core/Cargo.toml", errors[0])
            self.assertTrue(errors[1].startswith("crates/core/tests/lanes/lane.rs: "))
            self.assertTrue(errors[2].startswith("crates/core/tests/support/commented_out.rs: "))
            # Declared in the package's `it` target, the file passes; so does one with a target of its own.
            it.write_text(it.read_text() + "mod drift;\n")
            (tests / "support" / "commented_out.rs").unlink()
            metadata["packages"][0]["targets"].append(
                {"kind": ["test"], "name": "lanes", "src_path": str(tests / "lanes" / "lane.rs")})
            self.assertEqual(check(root, metadata), [])

    def test_feature_gated_test_must_be_covered_by_the_main_tier_list(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            render, game = root / "crates" / "render", root / "examples" / "game"
            (render / "tests").mkdir(parents=True)
            (render / "src" / "tests").mkdir(parents=True)
            (game / "tests" / "stage").mkdir(parents=True)
            seam = render / "tests" / "seam.rs"
            seam.write_text("#![cfg(feature = \"counters\")]\n#[test]\nfn seam() {}\n")
            unit = render / "src" / "tests" / "device.rs"
            unit.write_text("#[cfg(all(test, feature = \"counters\"))]\nfn probe() {}\n")
            stage = game / "tests" / "stage" / "main.rs"
            stage.write_text("#[cfg(\n    feature = \"probe\",\n)]\nmod gated;\n")
            (game / "tests" / "stage" / "gated.rs").write_text("#[test]\nfn gated() {}\n")
            fault = game / "tests" / "fault.rs"
            fault.write_text("#[test]\nfn fault() {}\n")
            plain = game / "tests" / "plain.rs"
            plain.write_text("#[test]\nfn plain() { assert!(cfg!(test)); }\n")
            on = game / "tests" / "on.rs"
            on.write_text("#[cfg(feature = \"always\")]\n#[test]\nfn on() {}\n")

            def target(name, path, required=()):
                return {"kind": ["test"], "name": name, "src_path": str(path), "required-features": list(required)}

            metadata = {"workspace_members": ["render", "game"], "packages": [
                {"id": "render", "name": "render", "manifest_path": str(render / "Cargo.toml"),
                 "features": {"counters": []}, "dependencies": [],
                 "targets": [{"kind": ["lib"], "name": "render", "src_path": str(render / "src" / "lib.rs")},
                             target("seam", seam)]},
                {"id": "game", "name": "game", "manifest_path": str(game / "Cargo.toml"),
                 "features": {"probe": [], "always": [], "default": ["always"]}, "dependencies": [
                     {"name": "render", "features": [], "optional": False, "target": None}],
                 "targets": [target("stage", stage), target("fault", fault, ["render/fault"]),
                             target("plain", plain), target("on", on)]}]}
            check = repo_check.check_feature_tests
            listed = ("render features=counters\n"
                      "game features=probe stage\n"
                      "game features=render/fault fault\n")
            self.assertEqual(check(root, metadata, listed), [])
            # A new gated file in a package listed whole is covered with no edit to the list.
            (render / "tests" / "zz_probe_gpu.rs").write_text("#[cfg(feature = \"counters\")]\n#[test]\nfn z() {}\n")
            metadata["packages"][0]["targets"].append(target("zz_probe_gpu", render / "tests" / "zz_probe_gpu.rs"))
            self.assertEqual(check(root, metadata, listed), [])
            # A new gated file in a package listed by binary names the file, the feature and the list.
            newer = game / "tests" / "newer.rs"
            newer.write_text("#[test]\n#[cfg(feature = \"probe\")]\nfn newer() {}\n")
            metadata["packages"][1]["targets"].append(target("newer", newer))
            errors = check(root, metadata, listed)
            self.assertEqual(len(errors), 1)
            self.assertIn("examples/game/tests/newer.rs:2", errors[0])
            self.assertIn("feature probe", errors[0])
            self.assertIn(".config/long-tests.txt", errors[0])
            self.assertEqual(check(root, metadata, listed.replace("probe stage", "probe stage newer")), [])
            # A required feature the list lacks fails; so does every gated file once its line goes.
            metadata["packages"][1]["targets"][2]["required-features"] = ["probe"]
            errors = check(root, metadata, listed.replace("probe stage", "probe stage newer"))
            self.assertEqual(len(errors), 1)
            self.assertIn("[[test]] plain: requires feature probe", errors[0])
            # Cargo builds a target only with all its required features, so one line must give them all.
            metadata["packages"][1]["targets"][2]["required-features"] = ["probe", "render/fault"]
            split = listed.replace("probe stage", "probe stage newer plain").replace("fault fault", "fault fault plain")
            self.assertEqual(len(check(root, metadata, split)), 1)
            self.assertEqual(check(root, metadata, split + "game features=probe,render/fault plain\n"), [])
            metadata["packages"][1]["targets"][2]["required-features"] = []
            bare = check(root, metadata, "")
            self.assertEqual(sorted(e.split(":")[0] for e in bare), sorted([
                "crates/render/src/tests/device.rs", "crates/render/tests/seam.rs",
                "crates/render/tests/zz_probe_gpu.rs", "examples/game/Cargo.toml [[test]] fault",
                "examples/game/tests/newer.rs", "examples/game/tests/stage/main.rs"]))
            # A library test module is covered only by a whole entry.
            self.assertTrue(any("src/tests/device.rs" in e
                                for e in check(root, metadata, "render features=counters seam zz_probe_gpu\n")))
            # A binary listed as not run, with its owner and failure, is accounted for; without them it is not.
            not_run = listed.replace("probe stage", "probe stage newer").replace("game features=render/fault fault\n", "")
            self.assertEqual(check(root, metadata, not_run + "game not-run fault features=render/fault owner=X # fails\n"), [])
            self.assertEqual(len(check(root, metadata, not_run + "game not-run fault features=render/fault owner=X\n")), 1)
            self.assertIn(".config/long-tests.txt:3: expected `<package> not-run",
                          repo_check.parse_main_tier_list(not_run + "game not-run fault features=render/fault\n")[2][0])
            # The list's own lines are checked against the workspace.
            wrong = check(root, metadata, listed + "game features=nothing stage\nnone features=x\ngame features=probe gone\n")
            self.assertIn(".config/long-tests.txt:4: game has no feature nothing", wrong)
            self.assertIn(".config/long-tests.txt:5: no workspace package none", wrong)
            self.assertIn(".config/long-tests.txt:6: game has no test binary gone", wrong)

    def test_feature_gated_library_test_code_must_be_covered(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source = root / "examples" / "game" / "src"
            (source / "host").mkdir(parents=True)
            lib = source / "lib.rs"
            lib.write_text("mod host;\n#[cfg(test)]\nmod checks;\n#[cfg(feature = \"probe\")]\npub fn probe() {}\n")
            # Production code behind a feature, a brace in a string, and a gated test in the inline module.
            scene = source / "host" / "scene.rs"
            scene.write_text(
                "#[cfg(feature = \"probe\")]\npub fn live() { let _ = \"}\"; }\n"
                "#[cfg(test)]\nmod tests {\n    fn brace() -> &'static str { \"{\" }\n"
                "    #[cfg(all(feature = \"probe\", not(target_arch = \"wasm32\")))]\n    #[test]\n    fn gated() {}\n}\n"
                "#[cfg(feature = \"other\")]\npub fn after() {}\n")
            (source / "host.rs").write_text("pub mod scene;\n")
            checks = source / "checks.rs"
            checks.write_text("#[test]\nfn plain() {}\n")
            metadata = {"workspace_members": ["game"], "packages": [
                {"id": "game", "name": "game", "manifest_path": str(root / "examples" / "game" / "Cargo.toml"),
                 "features": {"probe": [], "other": [], "off": []}, "dependencies": [],
                 "targets": [{"kind": ["rlib", "cdylib"], "name": "game", "src_path": str(lib)}]}]}
            check = repo_check.check_feature_tests
            # Production cfgs (lib.rs, `live`, `after`) are not test code; the inline gated test is.
            errors = check(root, metadata, "")
            self.assertEqual(len(errors), 1)
            self.assertIn("examples/game/src/host/scene.rs:6", errors[0])
            self.assertIn("feature probe", errors[0])
            self.assertIn("`lib`", errors[0])
            self.assertEqual(check(root, metadata, "game features=probe lib\n"), [])
            self.assertEqual(check(root, metadata, "game features=probe\n"), [])
            # Another binary's line does not cover the library's tests.
            metadata["packages"][0]["targets"].append(
                {"kind": ["test"], "name": "other_bin", "src_path": str(root / "examples" / "game" / "tests" / "o.rs")})
            self.assertEqual(len(check(root, metadata, "game features=probe other_bin\n")), 1)
            # A `#[cfg(test)] mod checks;` file is test code whole, and so is a `#[test]` item's cfg.
            checks.write_text("#[cfg(feature = \"other\")]\n#[test]\nfn gated() {}\n")
            lib.write_text(lib.read_text() + "#[test]\n#[cfg(feature = \"off\")]\nfn top() {}\n")
            errors = check(root, metadata, "game features=probe lib\n")
            self.assertEqual(sorted(e.split(":")[0] for e in errors),
                             ["examples/game/src/checks.rs", "examples/game/src/lib.rs"])
            # A feature under `not(...)` only turns a test off, so it needs no line.
            checks.write_text("#[cfg(not(feature = \"other\"))]\n#[test]\nfn ungated() {}\n")
            lib.write_text(lib.read_text().replace("cfg(feature = \"off\")", "cfg(test)"))
            self.assertEqual(check(root, metadata, "game features=probe lib\n"), [])
            self.assertIn(".config/long-tests.txt:1: game has no test binary gone",
                          check(root, metadata, "game features=probe gone\n"))

    def test_ios_runtime_closure_rejects_authoring_and_sqlite(self):
        tree = "v1-game-ios v0.1.0\nv1-game v0.1.0\nthrum-cook v0.1.0\nthrum-source v0.1.0\nrusqlite v0.40.2\nlibsqlite3-sys v0.38.0\n"
        with patch.object(repo_check.subprocess, "run") as run:
            run.return_value.returncode = 0
            run.return_value.stdout = tree
            errors = repo_check.check_ios_runtime_closure(Path("."), "1.99.1")
        self.assertEqual(len(errors), 4)
        self.assertTrue(all("iOS runtime includes authoring package" in error for error in errors))

    def test_unused_direct_dependency_requires_a_source_use_or_reasoned_macro_record(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            package_dir = root / "examples" / "v1-game"
            (package_dir / "src").mkdir(parents=True)
            manifest = package_dir / "Cargo.toml"
            manifest.write_text("[package]\nname='v1-game'\n")
            source = package_dir / "src" / "lib.rs"
            source.write_text("pub fn game() {}\n")
            metadata = {"workspace_members": ["game"], "packages": [{
                "id": "game", "name": "v1-game", "manifest_path": str(manifest),
                "dependencies": [{"name": "generated-runtime", "rename": None}],
            }]}
            policy = {"packages": {"v1-game": {}}}
            self.assertEqual(repo_check.check_unused_direct_dependencies(metadata, policy),
                             ["unused direct dependency: v1-game -> generated_runtime"])
            source.write_text("use generated_runtime::Bridge;\n")
            self.assertEqual(repo_check.check_unused_direct_dependencies(metadata, policy), [])
            source.write_text("pub fn game() {}\n")
            policy["packages"]["v1-game"]["implicit_dependencies"] = {
                "generated-runtime": "generated by the export macro"
            }
            self.assertEqual(repo_check.check_unused_direct_dependencies(metadata, policy), [])
            policy["packages"]["v1-game"]["implicit_dependencies"]["generated-runtime"] = ""
            self.assertIn("invalid implicit dependency record: v1-game -> generated-runtime",
                          repo_check.check_unused_direct_dependencies(metadata, policy))

    def test_external_dependency_requires_exact_version_and_review_record(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            manifest = root / "crates" / "core" / "Cargo.toml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text('[package]\nversion.workspace=true\nedition.workspace=true\nrust-version.workspace=true\npublish.workspace=true\n[lints]\nworkspace=true\n')
            dependency = {"name": "hecs", "path": None,
                          "source": "registry+https://github.com/rust-lang/crates.io-index",
                          "req": "^0.11.2", "kind": None, "target": None}
            metadata = {"workspace_members": ["core"], "packages": [
                {"id": "core", "name": "core", "manifest_path": str(manifest),
                 "dependencies": [dependency]}]}
            policy = {"packages": {"core": {"manifest": "crates/core/Cargo.toml",
                                            "allowed_workspace_dependencies": []}},
                      "external_dependencies": {}}
            self.assertIn("unregistered external dependency: core -> hecs",
                          repo_check.check_packages(root, metadata, policy))
            policy["external_dependencies"]["hecs"] = {
                "requirement": "=0.11.2", "license": "MIT OR Apache-2.0",
                "purpose": "private component storage", "target_cost": "review Wasm and native builds"}
            self.assertIn("external dependency must use declared exact version: hecs",
                          repo_check.check_packages(root, metadata, policy))
            dependency["req"] = "=0.11.2"
            self.assertEqual(repo_check.check_packages(root, metadata, policy), [])

    def test_stale_digest_is_not_silently_refreshed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            file = root / "docs/design/spec.md"
            file.parent.mkdir(parents=True)
            file.write_text("selected mechanism")
            ledger = {"hash": design_hashes.digest(file), "reviewed_hash": "historical"}
            before = copy.deepcopy(ledger)
            with patch.object(design_hashes, "current_slots", lambda value: [(value, "hash", "spec.md")]):
                self.assertEqual(design_hashes.check_hashes(root, ledger), [])
                file.write_text("changed mechanism")
                self.assertEqual(design_hashes.check_hashes(root, ledger), ["stale current design digest: spec.md"])
            self.assertEqual(ledger, before)


def load_tests(loader, suite, _pattern):
    """Keep shape fixtures in the ordinary ./dev check self-test gate too."""
    import test_shape_rules
    import test_shape_ratchet

    for module in (test_shape_rules, test_shape_ratchet):
        suite.addTests(loader.loadTestsFromModule(module))
    return suite


if __name__ == "__main__":
    unittest.main()
