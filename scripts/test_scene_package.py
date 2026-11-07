"""Scene folder resolution checks for ./dev scene."""

from pathlib import Path
import tempfile
import unittest
from unittest import mock

import scene_package
from scene_package import SceneError, resolve


def package(root, folder, name, binaries):
    manifest = Path(root) / folder / "Cargo.toml"
    manifest.parent.mkdir(parents=True, exist_ok=True)
    # The manifest's text is irrelevant: Cargo's metadata is the authority, so legal TOML that a
    # line scanner misreads (quoted keys, inline comments) cannot change the result.
    manifest.write_text('[package]\n"name" = "decoy" # name = "other"\n')
    targets = [{"name": binary, "kind": ["bin"]} for binary in binaries]
    targets.append({"name": name.replace("-", "_"), "kind": ["lib"]})
    return {"id": f"path+file://{manifest.parent}#{name}@0.1.0", "name": name,
            "manifest_path": str(manifest), "targets": targets}


class SceneResolution(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def metadata(self, *packages, members=None):
        ids = [p["id"] for p in packages] if members is None else members
        return {"packages": list(packages), "workspace_members": ids}

    def test_a_member_resolves_to_its_own_binary_name(self):
        scene = package(self.root, "examples/sunset", "thrum-sunset", ["sunset-window"])
        self.assertEqual(resolve(self.root, "examples/sunset/", self.metadata(scene)),
                         ("thrum-sunset", "sunset-window"))

    def test_a_folder_named_only_inside_another_member_path_is_not_a_member(self):
        # The old check searched Cargo.toml for the quoted folder, which a comment or a longer
        # member path could satisfy.
        other = package(self.root, "examples/sunset-two", "thrum-sunset-two", ["thrum-sunset-two"])
        (self.root / "examples/sunset").mkdir(parents=True)
        (self.root / "examples/sunset/Cargo.toml").write_text("")
        with self.assertRaisesRegex(SceneError, "is not a workspace member"):
            resolve(self.root, "examples/sunset", self.metadata(other))

    def test_a_listed_package_outside_the_members_is_refused(self):
        scene = package(self.root, "examples/sunset", "thrum-sunset", ["thrum-sunset"])
        with self.assertRaisesRegex(SceneError, "is not a workspace member"):
            resolve(self.root, "examples/sunset", self.metadata(scene, members=[]))

    def test_a_package_without_exactly_one_binary_is_refused(self):
        for binaries, found in [([], "none"), (["a", "b"], "a, b")]:
            scene = package(self.root, "examples/sunset", "thrum-sunset", binaries)
            with self.assertRaisesRegex(SceneError, f"builds {len(binaries)} binaries \\({found}\\)"):
                resolve(self.root, "examples/sunset", self.metadata(scene))

    def test_a_folder_without_a_manifest_is_refused(self):
        with self.assertRaisesRegex(SceneError, "has no Cargo.toml"):
            resolve(self.root, "examples/missing", self.metadata())

    def test_the_lockfile_is_left_alone_for_a_folder_that_does_not_resolve(self):
        with mock.patch.object(scene_package, "workspace_metadata", return_value=self.metadata()), \
                mock.patch.object(scene_package, "lockfile_current") as current, \
                mock.patch.object(scene_package.subprocess, "run") as run, \
                mock.patch("sys.stderr"):
            self.assertEqual(scene_package.main(["--lock", "examples/missing"], self.root), 1)
        current.assert_not_called()
        run.assert_not_called()

    def test_a_resolved_folder_updates_a_stale_lockfile_offline(self):
        scene = package(self.root, "examples/sunset", "thrum-sunset", ["thrum-sunset"])
        with mock.patch.object(scene_package, "workspace_metadata", return_value=self.metadata(scene)), \
                mock.patch.object(scene_package, "lockfile_current", return_value=False), \
                mock.patch.object(scene_package.subprocess, "run") as run, \
                mock.patch("sys.stderr"), mock.patch("sys.stdout"):
            self.assertEqual(scene_package.main(["--lock", "examples/sunset"], self.root), 0)
        self.assertEqual(run.call_args.args[0][1:], ["update", "--offline", "--workspace", "--quiet"])


if __name__ == "__main__":
    unittest.main()
