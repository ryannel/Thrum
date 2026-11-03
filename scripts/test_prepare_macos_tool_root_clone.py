"""Small macOS checks for exact-inventory APFS tool-root clones."""

import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from inventory_headless_tool_roots import inventory_root
import prepare_macos_tool_root_clone as clone_module
from prepare_macos_tool_root_clone import prepare_clone


@unittest.skipUnless(sys.platform == "darwin", "requires macOS clonefile")
class PrepareMacosToolRootClone(unittest.TestCase):
    def test_exact_clone_with_relative_links_and_private_write(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / "source"
            (source / "bin").mkdir(parents=True)
            (source / "bin/compiler").write_bytes(b"original tool bytes")
            (source / "bin/alias").symlink_to("compiler")
            (source / "empty").mkdir()
            expected = inventory_root(source)
            clone = base / "clone"
            self.assertEqual(prepare_clone(source, clone, expected), expected)
            self.assertEqual(inventory_root(clone), expected)
            self.assertEqual(os.readlink(clone / "bin/alias"), "compiler")
            (clone / "bin/compiler").write_bytes(b"modified clone bytes")
            self.assertEqual((source / "bin/compiler").read_bytes(), b"original tool bytes")

    def test_wrong_digest_removes_only_new_clone(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / "source"
            source.mkdir()
            (source / "tool").write_bytes(b"selected")
            expected = inventory_root(source)
            expected["sha256"] = "0" * 64
            clone = base / "clone"
            with self.assertRaisesRegex(ValueError, "differs from selected bytes"):
                prepare_clone(source, clone, expected)
            self.assertFalse(clone.exists())
            self.assertEqual((source / "tool").read_bytes(), b"selected")

    def test_existing_destination_is_not_touched(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / "source"
            source.mkdir()
            (source / "tool").write_bytes(b"selected")
            clone = base / "clone"
            clone.mkdir()
            marker = clone / "owner"
            marker.write_bytes(b"untouched")
            with self.assertRaisesRegex(ValueError, "destination absent"):
                prepare_clone(source, clone, inventory_root(source))
            self.assertEqual(marker.read_bytes(), b"untouched")

    def test_source_alias_and_absolute_link_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / "source"
            source.mkdir()
            (source / "tool").write_bytes(b"selected")
            (base / "source-alias").symlink_to(source, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "ordinary absolute directory"):
                prepare_clone(base / "source-alias", base / "clone", inventory_root(source))
            (source / "alias").symlink_to(source / "tool")
            # The inventory accepts contained absolute links, but cloning one
            # would leave a reference to the installed tree.
            with self.assertRaisesRegex(ValueError, "absolute tool-root link"):
                prepare_clone(source, base / "clone", inventory_root(source))
            self.assertFalse((base / "clone").exists())

    def test_unexpected_member_refuses_before_full_clone(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / "source"
            source.mkdir()
            (source / "tool").write_bytes(b"selected")
            expected = inventory_root(source)
            (source / "extra").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "exceeds expected files"):
                prepare_clone(source, base / "clone", expected)
            self.assertFalse((base / "clone").exists())

    def test_source_changes_during_clone_refuses_and_cleans_up(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / "source"
            source.mkdir()
            tool = source / "tool"
            tool.write_bytes(b"selected")
            real_clone = clone_module._clone_function()

            def change_after_clone(*args):
                result = real_clone(*args)
                tool.write_bytes(b"changed!")
                return result

            with patch.object(clone_module, "_clone_function", return_value=change_after_clone):
                with self.assertRaisesRegex(ValueError, "changed during clone"):
                    prepare_clone(source, base / "clone", inventory_root(source))
            self.assertFalse((base / "clone").exists())

    def test_special_file_refuses_without_opening_it(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / "source"
            source.mkdir()
            (source / "tool").write_bytes(b"selected")
            expected = inventory_root(source)
            os.mkfifo(source / "pipe")
            with self.assertRaisesRegex(ValueError, "unsupported tool-root entry"):
                prepare_clone(source, base / "clone", expected)
            self.assertFalse((base / "clone").exists())


if __name__ == "__main__":
    unittest.main()
