"""The campaign must witness actual rendering, not worker metadata writes."""
import json
from pathlib import Path
import tempfile
import unittest

from culling_campaign import completed_worker_rows


class WorkerProgress(unittest.TestCase):
    def test_metadata_and_unfinished_writes_do_not_count_as_rendering(self):
        scratch = Path(__file__).resolve().parents[1] / 'tmp'
        scratch.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as directory:
            root = Path(directory)
            worker = root / 'worker-0-0'
            worker.mkdir()
            path = worker / 'rows.jsonl'
            rows = [{'adapter': 'Apple M3 Max', 'mode': 'fixed'},
                    {'tick': 0, 'mode': 'Direct', 'whole_gpu_ns': 1000},
                    {'compared': 361, 'equal': True}]
            path.write_bytes(('\n'.join(map(json.dumps, rows)) + '\n').encode()
                             + b'{"tick":180,"mode":"Frust')
            self.assertEqual(completed_worker_rows(root), {'worker-0-0/rows.jsonl': 1})
            with path.open('ab') as stream:
                stream.write(b'um"}\n')
            self.assertEqual(completed_worker_rows(root), {'worker-0-0/rows.jsonl': 2})


if __name__ == '__main__':
    unittest.main()
