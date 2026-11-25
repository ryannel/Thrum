#!/usr/bin/env python3
"""Exercise the reference guard in Git repositories, retention and exact Chrome modes."""
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
from types import SimpleNamespace
from unittest.mock import patch

import exact_baselines as guard
from exact_baselines_native import run_matrix
from apple_tile_m1_compare_browser import IDENTITY, compare, compare_digest


class RepositoryCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git('init', '-b', 'main')
        for path, data in {'.gitignore': '/tmp/\n', 'refs/image': 'pixels',
                           'scripts/compare_v1_pixels.py': 'equal',
                           'scripts/exact_baselines.py': 'verifier',
                           'docs/plans/ARCHITECTURE.md': '# Exact baselines\n## Refresh allowed\n'}.items():
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(data)
        (self.root / '.config').mkdir()
        self.entry = {'id': 'fixture', 'owner': 'OWNER', 'reference': 'refs/',
                      'candidate': 'fresh', 'comparison_files': ['scripts/compare_v1_pixels.py',
                                                                'scripts/exact_baselines.py'],
                      'refresh': {'anchor': 'exact-baselines', 'module': 'OWNER', 'milestone': 'M1'}}
        self.data = {'baselines': [self.entry], 'retired': [], 'covered_paths': ['scripts/']}
        self.write_registry()
        self.commit('seed')
        self.repin()
        self.commit('initial pin')
        remote = self.root / 'tmp/remote.git'
        remote.parent.mkdir()
        self.git('init', '--bare', str(remote))
        self.git('remote', 'add', 'origin', str(remote))
        self.git('push', '-u', 'origin', 'main')
        self.base = self.git('rev-parse', 'HEAD')

    def git(self, *args):
        env = dict(os.environ, GIT_AUTHOR_NAME='Guard Test', GIT_AUTHOR_EMAIL='test@example.invalid',
                   GIT_COMMITTER_NAME='Guard Test', GIT_COMMITTER_EMAIL='test@example.invalid',
                   GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)
        return subprocess.check_output(['git', *args], cwd=self.root, stderr=subprocess.STDOUT,
                                       env=env).decode().strip()

    def commit(self, message):
        self.git('add', '.')
        self.git('commit', '--allow-empty', '-m', message)

    def write_registry(self):
        guard.save(self.root / guard.REGISTRY, self.data)

    def repin(self):
        for entry in self.data['baselines']:
            entry['pin'] = guard.pin(entry, guard.tree(self.root, 'HEAD'))
        self.write_registry()

    def refresh(self):
        for entry in self.data['baselines']:
            entry['refresh'] = {'anchor': 'refresh-allowed', 'module': 'OWNER', 'milestone': 'M2'}
        self.repin()
        self.commit('record refresh')

    def refuses(self, *details, identity='fixture'):
        with self.assertRaises(ValueError) as caught, contextlib.redirect_stdout(io.StringIO()):
            guard.guard(self.root)
        for detail in (identity, 'OWNER', 'rule 1', *details):
            self.assertIn(detail, str(caught.exception))
        self.assertEqual(self.git('rev-parse', 'refs/remotes/origin/main'), self.base)

    def passes(self):
        with contextlib.redirect_stdout(io.StringIO()):
            guard.guard(self.root)


class GuardTests(RepositoryCase):
    def test_initial_and_unrelated_change_pass(self):
        self.passes()
        (self.root / 'unrelated').write_text('allowed')
        self.commit('unrelated')
        self.passes()

    def test_changed_added_removed_and_comparison_files_refused(self):
        for path, operation in [('refs/image', 'change'), ('refs/new', 'add'),
                                ('refs/image', 'remove'), ('scripts/compare_v1_pixels.py', 'change'),
                                ('scripts/exact_baselines.py', 'change')]:
            with self.subTest(path=path, operation=operation):
                self.git('reset', '--hard', self.base)
                target = self.root / path
                if operation == 'remove':
                    target.unlink()
                    # Keep a second reference so a missing whole reference is not the only failure.
                    (self.root / 'refs/other').write_text('other')
                else:
                    target.write_text('changed')
                self.commit('mutant')
                self.refuses(path)

    def test_uncommitted_and_staged_protected_changes_refused(self):
        for path in ['refs/image', 'scripts/exact_baselines.py', guard.REGISTRY]:
            (self.root / path).write_text('dirty')
            self.refuses(path, 'uncommitted')
            self.git('add', path)
            self.refuses(path, 'uncommitted')
            self.git('reset', '--hard', self.base)

    def test_changed_pin_requires_changed_refresh(self):
        (self.root / 'refs/image').write_text('new pixels')
        self.commit('changed reference')
        self.repin()
        self.commit('pin only')
        self.refuses('refresh')
        self.refresh()
        self.passes()

    def test_missing_anchor_refused(self):
        self.entry['refresh']['anchor'] = 'not-a-ruling'
        self.write_registry()
        self.commit('invalid ruling')
        self.refuses('anchor')

    def test_missing_main_fails_closed(self):
        self.git('update-ref', '-d', 'refs/remotes/origin/main')
        with self.assertRaises(subprocess.CalledProcessError):
            guard.guard(self.root)


class RegisteredSetTests(RepositoryCase):
    def test_removal_without_retirement_refused(self):
        other = dict(self.entry, id='other')
        self.data['baselines'].append(other)
        self.commit('before addition')
        self.repin()
        self.write_registry()
        self.commit('two registered')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        self.base = self.git('rev-parse', 'HEAD')
        self.data['baselines'].remove(self.entry)
        self.write_registry()
        self.commit('remove only fixture')
        self.refuses('fixture', 'neither registered nor retired')

    def test_definition_change_requires_pin_and_refresh(self):
        self.entry['candidate'] = 'different'
        self.write_registry()
        self.commit('redefined')
        self.refuses('definition.candidate')
        self.refresh()
        self.passes()

    def test_new_baseline_requires_refresh(self):
        self.entry['id'] = 'new-baseline'
        self.data['retired'] = [dict(id='fixture', owner='OWNER', refresh=self.entry['refresh'])]
        del self.entry['refresh']
        self.commit('before addition')
        self.repin()
        self.commit('new without refresh')
        self.refuses('refresh', identity='new-baseline')
        self.refresh()
        self.passes()

    def test_retirement_requires_existing_anchor(self):
        self.data['baselines'] = []
        retired = {'id': 'fixture', 'owner': 'OWNER',
                   'refresh': {'anchor': 'missing', 'module': 'OWNER', 'milestone': 'M2'}}
        self.data['retired'] = [retired]
        self.write_registry()
        self.commit('invalid retirement')
        self.refuses('anchor')
        retired['refresh']['anchor'] = 'refresh-allowed'
        self.write_registry()
        self.commit('valid retirement')
        self.passes()

    def test_working_registry_cannot_hide_committed_removal(self):
        self.data['baselines'] = []
        self.write_registry()
        self.commit('remove')
        self.data['baselines'] = [self.entry]
        self.write_registry()
        self.refuses('neither registered nor retired')


class ScratchTests(unittest.TestCase):
    def test_bounds_and_preserves_latest_failure_and_in_progress(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            scratch = root / guard.SCRATCH
            for i in range(60):
                run = scratch / f'verify-{i:03}'
                run.mkdir(parents=True)
                guard.save(run / 'record.json', {'passed': i % 2 == 0, 'seconds': 1, 'error': 'failed'})
                (run / 'capture').mkdir()
                (run / 'capture/pixels').write_text('pixels')
            active = scratch / 'verify-999'
            active.mkdir()
            guard.save(active / 'record.json', {'passed': False})
            outside = root / 'outside'
            outside.mkdir()
            (outside / 'keep').write_text('keep')
            (scratch / 'verify-998').symlink_to(outside, target_is_directory=True)
            guard.prune_scratch(root)
            self.assertTrue((active / 'record.json').exists())
            self.assertTrue((scratch / 'verify-059/capture/pixels').exists())
            self.assertFalse((scratch / 'verify-057/capture').exists())
            self.assertFalse((scratch / 'verify-000').exists())
            self.assertEqual(len([p for p in scratch.glob('verify-*') if not p.is_symlink()]), 49)
            self.assertTrue((outside / 'keep').exists())
            guard.prune_scratch(root)  # idempotent

    def test_old_latest_failure_survives_record_limit_and_symlink_root_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            scratch = root / guard.SCRATCH
            for i in range(52):
                run = scratch / f'verify-{i:03}'
                run.mkdir(parents=True)
                guard.save(run / 'record.json', {'passed': i != 0, 'seconds': 1, 'error': 'failed'})
            guard.prune_scratch(root)
            self.assertTrue((scratch / 'verify-000/record.json').exists())
            self.assertEqual(len(list(scratch.iterdir())), 49)
            shutil.rmtree(scratch)
            scratch.symlink_to(root, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, 'inside this checkout'):
                guard.prune_scratch(root)


class MainCheckTests(unittest.TestCase):
    """`verify` without --full hands the question to the orchestrate script and returns its status."""

    def ask(self, script_body):
        with tempfile.TemporaryDirectory() as temp:
            script = Path(temp) / 'agent_loop.py'
            script.write_text(script_body)
            with patch.dict(os.environ, {'ORCHESTRATE_SCRIPT': str(script)}):
                return guard.covered_by_main(Path(temp))

    def test_statuses_pass_through(self):
        body = ('import sys\n'
                'assert sys.argv[1:4] == ["main-check", "exact-images", "--worktree"], sys.argv\n'
                'raise SystemExit({})\n')
        for status in [0, 1, 75]:
            self.assertEqual(self.ask(body.format(status)), status)

    def test_without_the_script_it_draws_nothing_and_fails(self):
        with patch.dict(os.environ), contextlib.redirect_stderr(io.StringIO()) as output:
            os.environ.pop('ORCHESTRATE_SCRIPT', None)
            with patch.object(guard, 'verify', side_effect=AssertionError('full run started')):
                self.assertEqual(guard.covered_by_main(), 2)
        self.assertIn('verify --full', output.getvalue())


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


class NativePublicationTests(unittest.TestCase):
    def test_next_capture_observes_completed_capture_release(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            matrix, records = root / 'matrix', root / 'records'
            records.mkdir()
            published = []
            owner = SimpleNamespace()

            def save(path, value):
                guard.save(path, value)
                published.append(json.loads(path.read_text()))

            def run(args):
                self.assertEqual(args.binary, root / 'binary')
                self.assertEqual(args.store, root / 'store')
                args.output.mkdir()
                capture = args.output / 'route-motion'
                capture.mkdir()
                guard.save(capture / 'manifest.json', {'complete': True})
                (capture / 'spool').write_bytes(b'completed capture')
                report = {'failures': [], 'runs': {'route-motion': {'complete': True}}, 'comparisons': {}}
                owner.save(args.output / 'manifest.json', report)
                # The next owner's preflight runs immediately, without a polling interval.
                self.assertFalse(capture.exists())
                self.assertEqual(published, [report])
                self.assertTrue((records / 'matrix-route-motion-records.json.gz').is_file())
                return 0

            owner.save, owner.run = save, run
            with patch('exact_baselines_native.importlib.import_module', return_value=owner):
                self.assertEqual(run_matrix('apple_tile_m2', root / 'binary', root / 'store', matrix, records), 0)
            self.assertIs(owner.save, save)

    def test_failed_capture_and_owner_exit_are_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            matrix, records = root / 'matrix', root / 'records'
            records.mkdir()
            owner = SimpleNamespace(save=guard.save)

            def run(args):
                args.output.mkdir()
                capture = args.output / 'failed'
                capture.mkdir()
                (capture / 'pixels').write_bytes(b'diagnostic source')
                report = {'failures': ['exact image changed'], 'runs': {'failed': {'complete': True}},
                          'comparisons': {'failed': {'equal': False}}}
                owner.save(args.output / 'manifest.json', report)
                self.assertTrue((capture / 'pixels').exists())
                return 1

            owner.run = run
            with patch('exact_baselines_native.importlib.import_module', return_value=owner):
                self.assertEqual(run_matrix('apple_tile_m1', root / 'binary', root / 'store', matrix, records), 1)
            self.assertIs(owner.save, guard.save)


class WasmIdentityTests(unittest.TestCase):
    def test_every_capture_reports_the_proving_bundle(self):
        self.verify_receipts('equal')

    def test_different_receipt_fails(self):
        self.verify_receipts('different')

    def test_missing_receipt_fails(self):
        self.verify_receipts('missing')

    def verify_receipts(self, outcome):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'reference').write_text('pixels')
            (root / 'target/release').mkdir(parents=True)
            (root / 'target/release/v1-game').write_text('native game')
            baselines = [dict(entry, reference='reference') for entry in guard.registry()['baselines']]
            chrome_ids = [entry['id'] for entry in baselines if 'scene' in entry]
            last = baselines[-1]
            bundle = {'v1_game_bg.wasm': b'proving artifact', 'capture.html': b'proving page'}
            hashes = {name: hashlib.sha256(data).hexdigest() for name, data in bundle.items()}
            expected = hashes['v1_game_bg.wasm']
            reported = {'equal': expected, 'different': hashlib.sha256(b'other module').hexdigest(),
                        'missing': None}[outcome]
            builds, captures, comparisons = [], [], []

            def child(command, **_):
                def option(name):
                    return command[command.index(name) + 1]

                if 'examples/v1-game/web/build.sh' in command:
                    self.assertEqual(command[0], 'env')
                    self.assertTrue(command[1].startswith('THRUM_V1_WEB_OUTPUT='))
                    self.assertEqual(command[-1], '--proving')
                    web = Path(command[1].split('=', 1)[1])
                    builds.append(web)
                    web.mkdir()
                    for name, data in bundle.items():
                        (web / name).write_bytes(data)
                elif 'scripts/exact_baselines_native.py' in command:
                    directory = Path(option('--output'))
                    directory.mkdir()
                    (directory / 'manifest.json').write_text('{"passed": true, "comparisons": {}}')
                elif 'scripts/capture_v1_browser.mjs' in command:
                    self.assertEqual([Path(option('--web-root'))], builds)
                    self.assertEqual(option('--wasm-receipt'), 'on')
                    directory = Path(option('--output'))
                    directory.mkdir()
                    captures.append(directory.name)
                    digest = reported if directory.name == last['id'] else expected
                    manifest = {} if digest is None else {'instantiated_wasm_sha256': digest}
                    (directory / 'manifest.json').write_text(json.dumps(manifest))
                elif 'scripts/apple_tile_m1_compare_browser.py' in command:
                    comparisons.append(Path(option('--candidate')).name)
                return subprocess.CompletedProcess(command, 0, stdout=b'')

            with patch.object(guard, 'registry', return_value={'baselines': baselines}), \
                    patch.object(guard, 'git', side_effect=lambda *a, **k: '' if a[0] == 'status' else 'revision'), \
                    patch.object(guard, 'content_key', return_value='test key'), \
                    patch.object(guard, 'git_version_command', return_value='fixture'), \
                    patch('run_v1_showcase.store_root', return_value='test root'), \
                    patch.object(guard.subprocess, 'run', side_effect=child), \
                    contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as stderr:
                code = guard.verify(root)
            record = json.loads(next(root.glob('tmp/baseline-guard/verify-*/record.json')).read_text())
            self.assertEqual(len(builds), 1)
            self.assertEqual(captures, chrome_ids)
            self.assertEqual(record['wasm_artifact_hashes'], hashes)
            passed = outcome == 'equal'
            self.assertIs(record['passed'], passed)
            self.assertEqual(code, 0 if passed else 1)
            self.assertEqual(comparisons, chrome_ids if passed else chrome_ids[:-1])
            for name in chrome_ids:
                comparison = record['comparisons'][name]
                self.assertEqual(comparison['instantiated_wasm_sha256'], reported if name == last['id'] else expected)
                self.assertIs(comparison['passed'], passed or name != last['id'])
            if not passed:
                error = record['error']
                self.assertIn(f"baseline {last['id']} owner {last['owner']}", error)
                self.assertIn('missing instantiated Wasm' if reported is None else 'differs from proving bundle', error)
                self.assertIn(error, stderr.getvalue())
                self.assertFalse(record.get('could_not_judge'))


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
