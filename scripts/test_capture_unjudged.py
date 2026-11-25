"""Exercise capture, native-matrix and full-verifier result paths with child stand-ins."""
import argparse
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import apple_tile_m0 as common
import apple_tile_m1 as matrix1
import apple_tile_m2 as matrix2
import capture_v1_runtime as capture
from capture_result import CaptureUnjudged
import exact_baselines as verifier
from exact_baselines_native import run_matrix


class CaptureExit(unittest.TestCase):
    def test_real_capture_command(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / 'engine'
            binary.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
case = os.environ['CAPTURE_TEST_OUTCOME']
if case in ('crash', 'expired-wait'): sys.exit(1)
if case == 'missing': sys.exit(0)
runtime = dict(complete=case == 'complete', backend='Metal', recipe='Portable',
    enabled_features='', pixel_source='Sdr', scenario='off', opportunities=[], images=[],
    interval_complete=True, ring_omissions=0, writer_omissions=0,
    writer_result={'Ok': {'bytes':0, 'encoded_bytes':0}})
if case == 'backend': runtime['backend']='Vulkan'
if case == 'ring': runtime['ring_omissions']=1
if case == 'writer': runtime['writer_result']={'Err':'writer deadline'}
if case == 'interval': runtime['interval_complete']=False
if case == 'overload': runtime['world_overload_recoveries']=[{'refused_ticks':20}]
if case == 'join': runtime['images']=[{'submission_id':99}]
if case == 'image-count': runtime['opportunities']=[{'submitted':True, 'submission_id':1}]
if case == 'source': runtime['pixel_source']='NativeHistory'
if case == 'encoded-bytes': runtime['writer_result']['Ok']['encoded_bytes']=1
if case == 'hash':
    raw = bytes(1920*1080*4)
    sys.stdout.buffer.write(raw)
    runtime.update(scenario='variants', spool_encoding='Raw',
        opportunities=[{'submitted':True, 'submission_id':1}],
        images=[{'submission_id':1, 'submit_ns':1, 'map_observed_ns':2}],
        writer_result={'Ok':{'bytes':len(raw), 'encoded_bytes':len(raw), 'frame_sha256':['f'*64]}})
pathlib.Path(os.environ['THRUM_CAPTURE_RUNTIME'], 'runtime.json').write_text(json.dumps(runtime))
''')
            binary.chmod(0o755)
            for case in ['complete', 'ring', 'writer', 'interval', 'overload', 'image-count', 'backend', 'join', 'source',
                         'encoded-bytes', 'hash', 'crash', 'missing', 'expired-wait']:
                with self.subTest(case=case):
                    output = root / case
                    result = subprocess.run([sys.executable, str(capture.ROOT / 'scripts/capture_v1_runtime.py'),
                        '--binary', str(binary), '--output', str(output), '--case', 'off', '--defer'],
                        env=dict(os.environ, CAPTURE_TEST_OUTCOME=case), capture_output=True, text=True, timeout=30)
                    expected = 0 if case == 'complete' else 75 if case in ['ring', 'writer', 'interval', 'overload', 'image-count'] else 1
                    self.assertEqual(result.returncode, expected, result.stderr)
                    manifest = json.loads((output / 'manifest.json').read_text())
                    if expected == 75:
                        self.assertFalse(manifest['complete'])
                        self.assertTrue(manifest['could_not_judge'])
                        self.assertTrue(manifest['incomplete_conditions'])
                        if case == 'overload':
                            self.assertIn('world overload recoveries: 1', manifest['incomplete_conditions'])
                    if case not in ('crash', 'missing', 'expired-wait'):
                        final = subprocess.run([sys.executable, str(capture.ROOT / 'scripts/capture_v1_runtime.py'),
                            '--output', str(output), '--finalize', '--verify-only'], capture_output=True, timeout=30)
                        self.assertEqual(final.returncode, expected, final.stderr)

    def test_process_deadline_is_failure(self):
        # Drive the actual process-deadline branch without waiting ninety seconds.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / 'engine'
            binary.write_text('#!/bin/sh\nexec sleep 30\n')
            binary.chmod(0o755)
            args = argparse.Namespace(output=root / 'output', finalize=False, source='sdr', binary=binary,
                store=None, case='off', mode='realtime', params=capture.ROOT / 'examples/v1-game/params/showcase-noon.params.json',
                profile=False, defer=True, verify_only=True)
            with patch.object(capture, 'permission_observation', return_value={'denied': True, 'error': None}), \
                    patch.object(capture.time, 'monotonic', side_effect=[0, 100]), \
                    self.assertRaises(SystemExit) as exited:
                capture.run(args)
            self.assertNotEqual(exited.exception.code, 75)

    def test_hash_corruption_is_failure(self):
        from capture_spool import decoded_frames
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'pixels'
            path.write_bytes(b'bad!')
            with self.assertRaises(ValueError) as error:
                list(decoded_frames(path, 'Raw', ['0' * 64], 4))
            self.assertNotIsInstance(error.exception, CaptureUnjudged)


class MatrixExit(unittest.TestCase):
    def test_both_real_matrix_paths_and_native_adapter(self):
        for owner in [matrix1, matrix2]:
            for outcome in ['complete', 'incomplete', 'bytes', 'identity', 'crash', 'missing', 'timeout']:
                with self.subTest(matrix=owner.__name__, outcome=outcome), tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    route = root / 'route.json'
                    route.write_text(json.dumps({'checkpoints': [], 'render_extent': [1920, 1080], 'cooked_root': 'fixture'}))
                    binary = root / 'binary'
                    binary.write_bytes(b'fixture')
                    options = []

                    def native(*args, **kwargs):
                        if outcome in ('bytes', 'identity'):
                            raise ValueError(f'{outcome} differs before motion')
                        return {'header': {'recipe': f"Ok({'AppleTile' if os.environ['THRUM_RECIPE'] == 'apple-tile' else 'Portable'})"}}

                    def motion(args, directory, name, selected):
                        options.append(selected)
                        if selected['mode'] == 'realtime':
                            if outcome == 'incomplete':
                                raise CaptureUnjudged(f'{name}: ring_omissions: 1')
                            if outcome == 'timeout':
                                raise subprocess.TimeoutExpired('capture', 180)
                            if outcome in ('crash', 'missing'):
                                raise ValueError(outcome)
                        return {}

                    with patch.multiple(owner, ROUTE=route, validate_store=lambda *a: None,
                            run_native=native, run_t2=motion, finish_t2=lambda *a, **k: {}), \
                            patch.object(matrix1, 'retained_comparison', return_value={}), \
                            patch.object(verifier, 'archive_completed_runs'):
                        code = run_matrix(owner.__name__, binary, root, root / 'output', root)
                    self.assertEqual(code, 0 if outcome == 'complete' else 75 if outcome == 'incomplete' else 1)
                    if outcome in ('bytes', 'identity'):
                        self.assertFalse(options, 'a prior comparison failure must stop before incomplete motion')
                    for selected in options:
                        self.assertEqual(selected.get('admission'), 'wait' if selected['mode'] == 'realtime' else None)
                    report = json.loads((root / 'output/manifest.json').read_text())
                    if code == 75:
                        self.assertIn('ring_omissions', report['incomplete_capture'])

    def test_real_capture_wrapper_child_status(self):
        for status in [0, 1, 75]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root / 'capture').mkdir()
                (root / 'capture/manifest.json').write_text(json.dumps({'capture_complete': status == 0,
                    'could_not_judge': status == 75, 'incomplete_conditions': 'ring_omissions: 3'}))
                args = argparse.Namespace(binary=root / 'binary', store=root)
                # The stand-in is an actual child; run_t2 reads its exit as in production.
                real_run = subprocess.run
                def child(*a, **k):
                    return real_run([sys.executable, '-c', f'raise SystemExit({status})'], **k)
                with patch.object(common.subprocess, 'run', side_effect=child):
                    if status:
                        with self.assertRaises(CaptureUnjudged if status == 75 else ValueError):
                            common.run_t2(args, root, 'capture', {})
                    else:
                        self.assertTrue(common.run_t2(args, root, 'capture', {})['capture_complete'])


class VerifierExit(unittest.TestCase):
    def test_latest_unjudged_and_latest_failure_keep_their_captures(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, fields in [('verify-1', {'error': 'old failure'}),
                                 ('verify-2', {'error': 'latest failure'}),
                                 ('verify-3', {'error': 'ring', 'could_not_judge': True}),
                                 ('verify-4', {'passed': True})]:
                directory = root / verifier.SCRATCH / name
                (directory / 'capture').mkdir(parents=True)
                (directory / 'record.json').write_text(json.dumps(dict(seconds=1, **fields)))
            verifier.prune_scratch(root)
            self.assertTrue((root / verifier.SCRATCH / 'verify-2/capture').exists())
            self.assertTrue((root / verifier.SCRATCH / 'verify-3/capture').exists())
            self.assertFalse((root / verifier.SCRATCH / 'verify-1/capture').exists())
            self.assertFalse((root / verifier.SCRATCH / 'verify-4/capture').exists())

    def test_full_verifier_exit_and_record(self):
        for outcome in ['incomplete', 'bytes', 'identity', 'crash', 'missing', 'timeout', 'unexpected75']:
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root / 'reference').write_text('pixels')
                (root / 'target/release').mkdir(parents=True)
                (root / 'target/release/v1-game').write_text('binary')
                entry = {'id': 'native', 'owner': 'APPLE-TILE', 'reference': 'reference',
                         'comparisons': ['scripts/apple_tile_m1.py']}
                real_run = subprocess.run
                def child(command, **kwargs):
                    if command[0] == 'env' and command[1].startswith('THRUM_V1_WEB_OUTPUT='):
                        web = Path(command[1].split('=', 1)[1])
                        web.mkdir()
                        (web / 'v1_game_bg.wasm').write_bytes(b'wasm')
                    if 'scripts/exact_baselines_native.py' in command:
                        directory = Path(command[command.index('--output') + 1])
                        directory.mkdir()
                        if outcome != 'missing':
                            (directory / 'manifest.json').write_text(json.dumps({'passed': False,
                                'could_not_judge': outcome == 'incomplete',
                                'incomplete_capture': 'portable-route-motion: writer deadline'}))
                        if outcome == 'timeout':
                            raise subprocess.TimeoutExpired(command, 3000)
                        code = 75 if outcome in ('incomplete', 'unexpected75') else 1
                    else:
                        code = 0
                    kwargs.pop('env', None)
                    return real_run([sys.executable, '-c', f'raise SystemExit({code})'], **kwargs)
                with patch.object(verifier, 'registry', return_value={'baselines': [entry]}), \
                        patch.object(verifier, 'git', side_effect=lambda *a, **k: '' if a[0] == 'status' else 'revision'), \
                        patch.object(verifier, 'content_key', return_value='key'), \
                        patch.object(verifier, 'git_version_command', return_value='fixture'), \
                        patch('run_v1_showcase.store_root', return_value='root'), \
                        patch.object(verifier.subprocess, 'run', side_effect=child):
                    code = verifier.verify(root)
                self.assertEqual(code, 75 if outcome == 'incomplete' else 1)
                record = json.loads(next(root.glob('tmp/baseline-guard/verify-*/record.json')).read_text())
                self.assertFalse(record['passed'])
                if code == 75:
                    self.assertTrue(record['could_not_judge'])
                    self.assertIn('portable-route-motion', record['incomplete_capture'])


if __name__ == '__main__':
    unittest.main()
