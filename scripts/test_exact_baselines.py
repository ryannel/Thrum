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
from types import SimpleNamespace
from unittest.mock import patch

import exact_baselines as guard
from exact_baselines_native import run_matrix
from apple_tile_m1_compare_browser import IDENTITY, compare, compare_digest
from exact_baselines_coverage import Coverage, check_build_reads


def package(root, folder, name, dependencies=''):
    directory = root / folder
    (directory / 'src').mkdir(parents=True, exist_ok=True)
    (directory / 'Cargo.toml').write_text(
        f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n' + dependencies)
    (directory / 'src/lib.rs').write_text('pub fn value() -> u32 { 1 }\n')


def workspace(root):
    (root / '.config').mkdir(exist_ok=True)
    config = guard.registry()
    config['built_packages'] = ['game', 'cooker']
    guard.save(root / guard.REGISTRY, config)
    (root / 'Cargo.toml').write_text('[workspace]\nresolver = "2"\nmembers = ["crates/game", "crates/cooker", "crates/linked", "crates/unlinked"]\n')
    package(root, 'crates/game', 'game', '[dependencies]\nlinked = { path = "../linked" }\n')
    package(root, 'crates/cooker', 'cooker')
    package(root, 'crates/linked', 'linked')
    package(root, 'crates/unlinked', 'unlinked')
    (root / '.gitignore').write_text('/tmp/\n/target/\n')


class WorkspaceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        workspace(self.root)
        self.git('init')
        self.commit()

    def git(self, *args):
        return GateTests.run_git(self, self.root, *args)

    def commit(self):
        self.git('add', '.')
        self.git('commit', '--allow-empty', '-m', 'fixture')

    def edit(self, path, text='changed\n'):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text)
        self.commit()

    def assert_key_edit(self, path, changes):
        before = guard.content_key(self.root)
        self.edit(path, (self.root / path).read_text() + '\n# edit\n')
        self.assertEqual(before != guard.content_key(self.root), changes, path)


class CoveredPathTests(WorkspaceTests):
    def test_listed_inputs_and_unlinked_files(self):
        for path in ['crates/linked/src/lib.rs', 'crates/linked/shader.wgsl',
                     'crates/game/assets/data', 'scripts/unused.py', 'Cargo.lock',
                     'docs/evidence/render/apple-tile-m0/native-baseline/ref',
                     'crates/unlinked/src/lib.rs', 'crates/unlinked/shader.wgsl',
                     'crates/unlinked/data', 'tools/nonpackage/file', '.cargo/config.toml']:
            if not (self.root / path).exists():
                self.edit(path, '# initial\n')
            self.assert_key_edit(path, not path.startswith('crates/unlinked/'))
        self.assert_key_edit('crates/unlinked/Cargo.toml', True)


class ClosureTests(WorkspaceTests):
    def test_all_dependency_kinds_and_transitive_edges(self):
        for declaration, joins in [('[dependencies]\nunlinked = { path = "../unlinked" }', True),
                                  ('[dependencies]\nunlinked = { path = "../unlinked", optional = true }', True),
                                  ('[target.\'cfg(windows)\'.dependencies]\nunlinked = { path = "../unlinked" }', True),
                                  ('[build-dependencies]\nunlinked = { path = "../unlinked" }', True),
                                  ('[dev-dependencies]\nunlinked = { path = "../unlinked" }', False)]:
            for parent in ['game', 'linked']:
                package(self.root, 'crates/' + parent, parent, declaration + '\n')
                if parent == 'linked':
                    package(self.root, 'crates/game', 'game', '[dependencies]\nlinked = { path = "../linked" }\n')
                self.commit()
                self.assert_key_edit('crates/unlinked/src/lib.rs', joins)

    def test_unknown_folders_nested_packages_and_explicit_registry(self):
        self.edit('crates/no-package/file')
        self.assert_key_edit('crates/no-package/file', True)
        package(self.root, 'crates/outsider', 'outsider', '[workspace]\n')
        (self.root / 'Cargo.toml').write_text(
            '[workspace]\nresolver = "2"\nmembers = ["crates/game", "crates/cooker", "crates/linked", "crates/unlinked", "crates/linked/nested"]\nexclude = ["crates/outsider"]\n')
        package(self.root, 'crates/linked/nested', 'nested')
        self.commit()
        self.assert_key_edit('crates/outsider/src/lib.rs', True)
        self.assert_key_edit('crates/linked/nested/src/lib.rs', True)
        self.edit('crates/unlinked/restored/file')
        config = guard.registry(self.root)
        config['covered_paths'].append('crates/unlinked/restored/')
        guard.save(self.root / guard.REGISTRY, config)
        self.commit()
        self.assert_key_edit('crates/unlinked/restored/file', True)

    def test_linked_package_outside_listed_paths(self):
        package(self.root, 'outside', 'outside')
        (self.root / 'Cargo.toml').write_text('[workspace]\nresolver = "2"\nmembers = ["crates/*", "outside"]\n')
        self.commit()
        self.assert_key_edit('outside/src/lib.rs', False)
        package(self.root, 'crates/game', 'game', '[dependencies]\noutside = { path = "../../outside" }\n')
        self.commit()
        self.assert_key_edit('outside/src/lib.rs', True)


class FailClosedTests(WorkspaceTests):
    def test_bad_workspace(self):
        self.edit('Cargo.toml', 'invalid toml')
        with self.assertRaisesRegex(ValueError, 'Cargo cannot load workspace'):
            guard.gate(self.root)

    def test_missing_built_package(self):
        config = guard.registry(self.root)
        config['built_packages'] = ['absent']
        guard.save(self.root / guard.REGISTRY, config)
        self.commit()
        with self.assertRaisesRegex(ValueError, 'built package.*absent'):
            guard.gate(self.root)

    def test_missing_registry_and_base(self):
        (self.root / guard.REGISTRY).unlink()
        with self.assertRaises(OSError):
            guard.gate(self.root)
        self.git('checkout', '--', guard.REGISTRY)
        with self.assertRaises(subprocess.CalledProcessError):
            guard.gate(self.root)

    def test_external_linked_package(self):
        with tempfile.TemporaryDirectory() as temp:
            external = Path(temp)
            package(external, 'external', 'external', '[workspace]\n')
            package(self.root, 'crates/game', 'game',
                    f'[dependencies]\nexternal = {{ path = "{external}/external" }}\n')
            self.commit()
            with self.assertRaisesRegex(ValueError, 'linked package outside repository'):
                guard.gate(self.root)


class BuildReadTests(WorkspaceTests):
    def test_real_build_uncovered_include_and_registry_cure(self):
        (self.root / 'crates/unlinked/data with spaces.txt').write_text('input')
        (self.root / 'crates/game/src/lib.rs').write_text(
            'pub const DATA: &str = include_str!("../../unlinked/data with spaces.txt");\n')
        self.commit()
        command = ['cargo', 'build', '--offline', '-p', 'game']
        subprocess.run(command, cwd=self.root, check=True, capture_output=True)
        records = ['target/debug/libgame.d']
        with self.assertRaisesRegex(ValueError, 'uncovered file crates/unlinked/data with spaces.txt.*record'):
            check_build_reads(self.root, Coverage(self.root, guard.registry(self.root)), records)
        config = guard.registry(self.root)
        config['covered_paths'].append('crates/unlinked/data with spaces.txt')
        guard.save(self.root / guard.REGISTRY, config)
        self.commit()
        subprocess.run(command, cwd=self.root, check=True, capture_output=True)
        result = check_build_reads(self.root, Coverage(self.root, config), records)
        self.assertIn('crates/unlinked/data with spaces.txt', result[0]['repository_inputs'])
        self.assert_key_edit('crates/unlinked/data with spaces.txt', True)
        with self.assertRaisesRegex(ValueError, 'missing/unreadable.*absent.d'):
            check_build_reads(self.root, Coverage(self.root, config), ['target/debug/absent.d'])

    def test_unreadable_depfile_refuses(self):
        self.edit('empty.d', 'not a depfile')
        with self.assertRaisesRegex(ValueError, 'no inputs'):
            check_build_reads(self.root, Coverage(self.root, guard.registry(self.root)), ['empty.d'])


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
        workspace(self.writer)
        (self.writer / 'crates/one').write_text('one')
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


class UnlinkedRebaseTests(GateTests):
    def test_unlinked_rebase_then_manifest_links_package(self):
        (self.builder / 'crates/two').write_text('builder')
        self.commit(self.builder, 'covered change')
        self.seed_proof()
        key = guard.content_key(self.builder)
        (self.writer / 'crates/unlinked/src/lib.rs').write_text('pub fn new() {}\n')
        self.commit(self.writer, 'unlinked source')
        self.run_git(self.writer, 'push', 'origin', 'main')
        self.assertTrue(self.land())
        self.assertEqual(key, guard.content_key(self.builder))
        # Keep a covered branch delta after the first successful push.
        (self.builder / 'crates/two').write_text('builder second')
        self.commit(self.builder, 'second covered change')
        self.seed_proof()
        self.run_git(self.writer, 'pull', '--ff-only')
        package(self.writer, 'crates/game', 'game', '[dependencies]\nunlinked = { path = "../unlinked" }\n')
        self.commit(self.writer, 'link formerly unlinked package')
        self.run_git(self.writer, 'push', 'origin', 'main')
        self.assertFalse(self.land())


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
