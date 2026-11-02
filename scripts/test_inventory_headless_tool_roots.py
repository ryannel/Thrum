"""Small content and alias checks for cold native tool-root inventories."""

import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from inventory_headless_tool_roots import inventory_root, inventory_roots


@unittest.skipUnless(os.name == "posix", "tool-root inventory requires POSIX")
class InventoryHeadlessToolRoots(unittest.TestCase):
    def test_content_and_directory_membership_change_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "tool"
            root.mkdir()
            (root / "bin").mkdir()
            tool = root / "bin/compiler"
            tool.write_bytes(b"tool A")
            first = inventory_root(root)
            self.assertEqual(first, inventory_root(root))
            tool.write_bytes(b"tool B")
            second = inventory_root(root)
            self.assertNotEqual(first["sha256"], second["sha256"])
            (root / "empty").mkdir()
            self.assertNotEqual(second["sha256"], inventory_root(root)["sha256"])

    def test_internal_symlink_is_recorded_and_escape_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / "tool"
            root.mkdir()
            (root / "compiler").write_bytes(b"tool")
            alias = root / "alias"
            alias.symlink_to("compiler")
            self.assertEqual(inventory_root(root)["symlinks"], 1)
            alias.unlink()
            alias.symlink_to(root / "compiler")
            self.assertEqual(inventory_root(root)["symlinks"], 1)
            alias.unlink()
            alias.symlink_to(parent / "outside")
            (parent / "outside").write_bytes(b"outside")
            with self.assertRaisesRegex(ValueError, "escapes selected root"):
                inventory_root(root)

    def test_file_replaced_by_escaping_symlink_cannot_be_hashed(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / "tool"
            root.mkdir()
            leaf = root / "compiler"
            leaf.write_bytes(b"inside")
            outside = parent / "outside"
            outside.write_bytes(b"outside")
            original_stat = os.stat
            swapped = False

            def swap_after_classification(path, *args, **kwargs):
                nonlocal swapped
                result = original_stat(path, *args, **kwargs)
                if path == "compiler" and kwargs.get("dir_fd") is not None and not swapped:
                    swapped = True
                    leaf.unlink()
                    leaf.symlink_to(outside)
                return result

            with patch("inventory_headless_tool_roots.os.stat", side_effect=swap_after_classification):
                with self.assertRaises((OSError, ValueError)):
                    inventory_root(root)
            self.assertTrue(swapped)

    def test_file_replaced_by_fifo_refuses_without_waiting_for_writer(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "tool"
            root.mkdir()
            leaf = root / "compiler"
            leaf.write_bytes(b"inside")
            original_stat = os.stat
            swapped = False

            def swap_after_classification(path, *args, **kwargs):
                nonlocal swapped
                result = original_stat(path, *args, **kwargs)
                if path == "compiler" and kwargs.get("dir_fd") is not None and not swapped:
                    swapped = True
                    leaf.unlink()
                    os.mkfifo(leaf)
                return result

            started = time.monotonic()
            with patch("inventory_headless_tool_roots.os.stat", side_effect=swap_after_classification):
                with self.assertRaises(ValueError):
                    inventory_root(root)
            self.assertTrue(swapped)
            self.assertLess(time.monotonic() - started, 1)

    def test_duplicate_roots_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "distinct named tool roots"):
                inventory_roots({"rust": root, "sdk": root})


if __name__ == "__main__":
    unittest.main()
