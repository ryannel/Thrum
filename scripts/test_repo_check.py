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
