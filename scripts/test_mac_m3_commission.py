"""M3 cannot spend its two-arm commission on repeated/diagnostic recordings."""
import argparse
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import qualify_mac_m2


class M3CommissionTests(unittest.TestCase):
    def test_exported_and_failed_arms_both_refuse_a_second_recording(self):
        args = argparse.Namespace(post_apple_tile_m3=True, presented_minimal=True,
                                  presented_deferred=True)
        for status in ('exported; attribution and full-pan join unjudged',
                       'failed; no unchanged retry', 'started'):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                ledger = root / 'tmp/mac-m3/recording-commission.json'
                ledger.parent.mkdir(parents=True)
                ledger.write_text(json.dumps({'attempts': [
                    {'arm': '/earlier/output/off-traced', 'status': status}]}))
                with patch.object(qualify_mac_m2, 'ROOT', root), \
                     patch.object(qualify_mac_m2.shutil, 'disk_usage',
                                  return_value=SimpleNamespace(free=32 * 1024**3)):
                    with self.assertRaisesRegex(RuntimeError, 'one recording per traced arm'):
                        qualify_mac_m2.collect(args, root, 'off-traced', 'off',
                                               traced=True, presented=True)
                self.assertFalse((root / 'off-traced').exists())


if __name__ == '__main__':
    unittest.main()
