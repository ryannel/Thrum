"""Exercise game selection and digest rejection through the match command."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("session_match.py")


@unittest.skipUnless(os.name == "posix", "the Mac program fixture uses an executable script")
class GameSelection(unittest.TestCase):
    def test_selects_the_requested_program_and_rejects_a_divergent_replica(self):
        with tempfile.TemporaryDirectory() as scratch:
            directory = Path(scratch)
            program = directory / "game-match"
            output = directory / "proof.json"
            program.write_text("""#!/usr/bin/env python3
import json
import sys
assert sys.argv[1:] == ["memory", "harsh", "2"]
world = {"tick": 2, "rule_set": "fe" * 32, "digests": [[1, "first"], [2, "second"]]}
print(json.dumps({"digest_interval": 1, "host": world, "players": [world]}))
""")
            program.chmod(0o700)
            command = [sys.executable, SCRIPT, "memory", "game", "--program", program,
                       "--profile", "harsh", "--ticks", "2", "--output", output]
            subprocess.run(command, check=True, capture_output=True, text=True)
            proof = json.loads(output.read_text())
            self.assertEqual(proof["match"]["host"]["rule_set"], "fe" * 32)
            self.assertEqual(proof["program"], str(program.resolve()))

            output.unlink()
            program.write_text(program.read_text().replace(
                "[world]", "[dict(world, digests=[[1, 'first'], [2, 'different']])]"))
            rejected = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("replica did not match the host", rejected.stderr)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
