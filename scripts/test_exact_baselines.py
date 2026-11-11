#!/usr/bin/env python3
"""Exercise proof gating through real fetch/rebase/push, and exact Chrome modes."""
import contextlib
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

import exact_baselines as guard
from apple_tile_m1_compare_browser import IDENTITY, compare, compare_digest


class GateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.remote = self.base / 'remote.git'
        self.run_git(self.base, 'init', '--bare', str(self.remote))
        self.writer = self.base / 'writer'
        self.run_git(self.base, 'clone', str(self.remote), str(self.writer))
        self.run_git(self.writer, 'checkout', '-b', 'main')
        (self.writer / '.config').mkdir()
        shutil.copyfile(guard.ROOT / guard.REGISTRY, self.writer / guard.REGISTRY)
        (self.writer / 'crates').mkdir()
        (self.writer / 'crates/one').write_text('one')
        (self.writer / '.gitignore').write_text('/tmp/\n')
        self.commit(self.writer, 'initial')
        self.run_git(self.writer, 'push', '-u', 'origin', 'main')
        self.builder = self.base / 'builder'
        self.run_git(self.base, 'clone', '-b', 'main', str(self.remote), str(self.builder))
        self.run_git(self.builder, 'checkout', '-b', 'build/test')

    def run_git(self, root, *args):
        env = dict(os.environ, GIT_AUTHOR_NAME='Guard Test', GIT_AUTHOR_EMAIL='test@example.invalid',
                   GIT_COMMITTER_NAME='Guard Test', GIT_COMMITTER_EMAIL='test@example.invalid',
                   GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)
        return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.STDOUT, env=env).decode().strip()

    def commit(self, root, name):
        self.run_git(root, 'add', '.')
        self.run_git(root, 'commit', '-m', name)

    def seed_proof(self):
        # Fixture for a completed verifier; real device proof is the M1 verifier run.
        path = guard.proof_path(self.builder)
        path.parent.mkdir(parents=True, exist_ok=True)
        guard.save(path, {'passed': True, 'content_key': guard.content_key(self.builder),
                          'baselines': [b['id'] for b in guard.registry(self.builder)['baselines']]})

    def land(self):
        """Same integration order as land, against only this disposable remote."""
        before = self.run_git(self.base, '--git-dir', str(self.remote), 'rev-parse', 'main')
        self.run_git(self.builder, 'fetch', 'origin', 'main')
        self.run_git(self.builder, 'rebase', 'origin/main')
        with contextlib.redirect_stdout(io.StringIO()):
            try:
                guard.gate(self.builder)
            except ValueError:
                self.assertEqual(before, self.run_git(self.base, '--git-dir', str(self.remote), 'rev-parse', 'main'))
                return False
        self.run_git(self.builder, 'push', 'origin', 'HEAD:main')
        self.assertEqual(self.run_git(self.builder, 'rev-parse', 'HEAD'),
                         self.run_git(self.base, '--git-dir', str(self.remote), 'rev-parse', 'main'))
        return True

    def test_no_covered_change_passes_without_proof(self):
        (self.builder / 'notes.md').write_text('documentation')
        self.commit(self.builder, 'docs')
        self.assertTrue(self.land())

    def test_covered_change_refuses_until_exact_proof(self):
        (self.builder / 'crates/one').write_text('changed')
        self.commit(self.builder, 'covered')
        self.assertFalse(self.land())
        self.seed_proof()
        self.assertTrue(self.land())

    def test_covered_rebase_invalidates_proof(self):
        (self.builder / 'crates/two').write_text('builder')
        self.commit(self.builder, 'builder covered')
        self.seed_proof()
        key = guard.content_key(self.builder)
        (self.writer / 'crates/one').write_text('upstream')
        self.commit(self.writer, 'upstream covered')
        self.run_git(self.writer, 'push', 'origin', 'main')
        self.assertFalse(self.land())
        self.assertNotEqual(key, guard.content_key(self.builder))
        self.seed_proof()
        self.assertTrue(self.land())

    def test_uncovered_rebase_preserves_proof(self):
        (self.builder / 'crates/two').write_text('builder')
        self.commit(self.builder, 'builder covered')
        self.seed_proof()
        key = guard.content_key(self.builder)
        (self.writer / 'notes.md').write_text('upstream docs')
        self.commit(self.writer, 'upstream docs')
        self.run_git(self.writer, 'push', 'origin', 'main')
        self.assertTrue(self.land())
        self.assertEqual(key, guard.content_key(self.builder))

    def test_missing_base_and_dirty_inputs_fail_closed(self):
        self.run_git(self.builder, 'update-ref', '-d', 'refs/remotes/origin/main')
        with self.assertRaises(subprocess.CalledProcessError):
            guard.gate(self.builder)
        (self.builder / 'crates/one').write_text('dirty')
        with self.assertRaisesRegex(ValueError, 'uncommitted'):
            guard.gate(self.builder)
        with self.assertRaisesRegex(ValueError, 'dirty tree'):
            guard.verify(self.builder)


class RetentionTests(unittest.TestCase):
    def test_only_published_successful_captures_are_released(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            matrix, output = root / 'matrix', root / 'records'
            matrix.mkdir(); output.mkdir()
            for name in ['compared', 'motion', 'in-progress']:
                (matrix / name).mkdir()
                guard.save(matrix / name / 'manifest.json', {'source_hashes': ['original']})
                (matrix / name / 'pixels').write_bytes(b'reconstructible')
            report = {'failures': [], 'runs': {'compared': {}, 'motion': {'complete': True}, 'in-progress': {}},
                      'comparisons': {'compared-checkpoint': {'equal': True}}}
            guard.save(matrix / 'manifest.json', report)
            guard.archive_completed_runs(matrix, output)
            self.assertFalse((matrix / 'compared').exists())
            self.assertFalse((matrix / 'motion').exists())
            self.assertTrue((matrix / 'in-progress/pixels').exists())
            archived = json.loads(gzip.decompress((output / 'matrix-motion-records.json.gz').read_bytes()))
            self.assertEqual(archived['manifest.json']['source_hashes'], ['original'])
            report['runs']['in-progress']['complete'] = True
            report['failures'] = ['comparison failed']
            guard.save(matrix / 'manifest.json', report)
            guard.archive_completed_runs(matrix, output)
            self.assertTrue((matrix / 'in-progress/pixels').exists())


class ChromeTests(unittest.TestCase):
    def test_digest_and_byte_modes_and_failures(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            reference, candidate = root / 'reference', root / 'candidate'
            reference.mkdir(); candidate.mkdir()
            raw = b'RGBA'
            rows = []
            for ordinal in range(481):
                row = {key: 1 for key in IDENTITY}
                row.update(ordinal=ordinal, file=f'{ordinal}.gz', rgba_sha256=hashlib.sha256(raw).hexdigest(),
                           pixel_source='scene-linear', exposure_gpu_words=[0] * 8)
                rows.append(row)
                for directory in [reference, candidate]:
                    (directory / row['file']).write_bytes(gzip.compress(raw))
            manifest = {'complete': True, 'mode': 'fixed', 'records': rows}
            for directory in [reference, candidate]:
                guard.save(directory / 'manifest.json', manifest)
            retained = root / 'reference.json.gz'
            retained.write_bytes(gzip.compress(json.dumps(manifest).encode()))
            with contextlib.redirect_stdout(io.StringIO()):
                for linear in [False, True]:
                    compare(reference, candidate, root / 'bytes.json', linear)
                    compare_digest(retained, candidate, root / 'digests.json', linear)
                self.assertEqual(json.loads((root / 'digests.json').read_text())['frames'], 481)
                # The last record must be inspected, and hashing must read frame bytes.
                (candidate / '480.gz').write_bytes(gzip.compress(b'FAIL'))
                with self.assertRaisesRegex(ValueError, 'source hash verification failed'):
                    compare_digest(retained, candidate, root / 'out.json', False)
                manifest['records'][-1]['rgba_sha256'] = hashlib.sha256(b'FAIL').hexdigest()
                guard.save(candidate / 'manifest.json', manifest)
                with self.assertRaisesRegex(ValueError, 'decoded image changed'):
                    compare_digest(retained, candidate, root / 'out.json', False)
                with self.assertRaisesRegex(ValueError, 'raw output changed|source hash verification failed'):
                    compare(reference, candidate, root / 'out.json', False)
                (candidate / '480.gz').write_bytes(gzip.compress(raw))
                manifest['records'][-1]['tick'] = 9
                guard.save(candidate / 'manifest.json', manifest)
                with self.assertRaisesRegex(ValueError, 'identity changed'):
                    compare_digest(retained, candidate, root / 'out.json', True)
                retained.write_bytes(b'not a gzip stream')
                with self.assertRaisesRegex(ValueError, 'unreadable retained reference'):
                    compare_digest(retained, candidate, root / 'out.json', False)
                retained.unlink()
                with self.assertRaisesRegex(ValueError, 'unreadable retained reference'):
                    compare_digest(retained, candidate, root / 'out.json', False)


if __name__ == '__main__':
    unittest.main(verbosity=2)
