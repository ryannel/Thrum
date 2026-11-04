"""Adversarial fixtures for checks that guard subsequent agent contributions."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import design_hashes
import repo_check


class RepositoryChecks(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
