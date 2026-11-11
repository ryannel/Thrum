#!/usr/bin/env python3
"""Run ONLY under the source repository's gpu lock, outside the sandbox.

python3 docs/evidence/BASELINE-GUARD/M1/reproduce_refusals.py
Requires a clean, committed source and a real full verifier proof for its key.
Uses tmp/baseline-guard/refusals/ and a local bare remote; never pushes to the
source repository's remote. The one scratch builder reuses this task's target
sequentially; the source's next verifier rebuilds its own artifacts.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / 'scripts'))
import exact_baselines as guard

LOOP = '/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py'
OUT = ROOT / 'tmp/baseline-guard/refusals'
ENV = dict(os.environ, GIT_AUTHOR_NAME='Baseline Guard Experiment', GIT_AUTHOR_EMAIL='guard@example.invalid',
           GIT_COMMITTER_NAME='Baseline Guard Experiment', GIT_COMMITTER_EMAIL='guard@example.invalid', RUSTC_WRAPPER='')


def git(root, *args):
    return subprocess.check_output(['git', *args], cwd=root, env=ENV, stderr=subprocess.STDOUT).decode().strip()


def run(root, name, command):
    start = time.monotonic()
    with (OUT / (name + '.log')).open('w') as log:
        result = subprocess.run(command, cwd=root, env=ENV, stdout=log, stderr=subprocess.STDOUT)
    print(name, result.returncode, time.monotonic() - start, flush=True)
    return {'command': command, 'exit': result.returncode, 'seconds': time.monotonic() - start}


def main():
    OUT.mkdir(parents=True, exist_ok=False)
    base = git(ROOT, 'rev-parse', 'HEAD')
    proof = json.loads(guard.proof_path(ROOT).read_text())
    if not proof['passed'] or proof['content_key'] != guard.content_key(ROOT):
        raise ValueError('a genuine source verifier proof is required')
    remote = OUT / 'remote.git'
    git(ROOT, 'clone', '--bare', '--no-hardlinks', str(ROOT), str(remote))
    git(ROOT, '--git-dir', str(remote), 'update-ref', 'refs/heads/main', base)
    git(ROOT, '--git-dir', str(remote), 'symbolic-ref', 'HEAD', 'refs/heads/main')
    builder = OUT / 'builder'
    git(ROOT, 'clone', '-b', 'main', str(remote), str(builder))
    git(builder, 'checkout', '-b', 'build/mutants')
    (builder / 'target').symlink_to(ROOT / 'target', target_is_directory=True)
    report = {'base': base, 'remote': str(remote), 'source_remote_used': False, 'cases': {}}
    scene = 'examples/v1-game/src/host/scene_source.rs'
    mutations = {
        'jitter': (scene, '.with_jitter_sequence(0, [640, 360]);', '.with_jitter_phase(0, [640, 360]);'),
        'night-only': ('examples/v1-game/params/showcase-night-upslope.params.json', '"sky.solar_time": 0', '"sky.solar_time": 6'),
        'chrome-only': (scene, '.with_jitter_sequence(0, [640, 360]);',
                        '.with_jitter_sequence(0, [640, 360]);\n        #[cfg(target_arch = "wasm32")]\n        let view = view.with_jitter_phase(0, [640, 360]);'),
    }
    for name, (file, before, after) in mutations.items():
        git(builder, 'reset', '--hard', base)
        path = builder / file
        text = path.read_text()
        if text.count(before) != 1:
            raise ValueError('mutant anchor changed')
        path.write_text(text.replace(before, after))
        (OUT / (name + '.patch')).write_text(git(builder, 'diff', '--', file) + '\n')
        git(builder, 'add', file)
        git(builder, 'commit', '-m', 'scratch mutant ' + name)
        case = {'revision': git(builder, 'rev-parse', 'HEAD')}
        case['verify'] = run(builder, name + '-verify', ['python3', 'scripts/exact_baselines.py', 'verify'])
        if case['verify']['exit'] == 0 or guard.proof_path(builder).exists():
            raise ValueError('mutant incorrectly obtained a proof')
        records = sorted((builder / guard.SCRATCH).glob('verify-*/record.json'))
        verifier_record = json.loads(records[-1].read_text())
        guard.save(OUT / (name + '-verifier-record.json'), verifier_record)
        if 'baseline ' not in verifier_record['error'] or 'owner APPLE-TILE' not in verifier_record['error']:
            raise ValueError('mutant failure did not name the baseline and owner')
        case['failure'] = verifier_record['error']
        for compact in records[-1].parent.glob('*.gz'):
            shutil.copyfile(compact, OUT / (name + '-' + compact.name))
        previous = git(ROOT, '--git-dir', str(remote), 'rev-parse', 'main')
        case['land'] = run(builder, name + '-land', ['python3', LOOP, 'land', '--worktree', '.'])
        log = (OUT / (name + '-land.log')).read_text()
        if '"result": "FAILED"' not in log or 'no exact-baseline proof' not in log:
            raise ValueError('land did not refuse the missing proof')
        case['remote_before'] = previous
        case['remote_after'] = git(ROOT, '--git-dir', str(remote), 'rev-parse', 'main')
        if case['remote_before'] != case['remote_after']:
            raise ValueError('scratch remote moved on refusal')
        report['cases'][name] = case
        guard.save(OUT / 'report.json', report)
        # Retain compact manifest/result/logs above; discard only owned failed capture bulk.
        shutil.rmtree(builder / guard.SCRATCH)
    git(builder, 'reset', '--hard', base)
    (builder / 'guard-proof-note.md').write_text('Proved content, documentation-only branch commit.\n')
    git(builder, 'add', 'guard-proof-note.md')
    git(builder, 'commit', '-m', 'scratch proved branch')
    proof_file = guard.proof_path(builder)
    proof_file.parent.mkdir(parents=True, exist_ok=True)
    guard.save(proof_file, proof)
    guard.gate(builder)
    writer = OUT / 'writer'
    git(ROOT, 'clone', '-b', 'main', str(remote), str(writer))
    (writer / 'scripts/upstream-covered.txt').write_text('Independent upstream covered content.\n')
    git(writer, 'add', 'scripts/upstream-covered.txt')
    git(writer, 'commit', '-m', 'scratch independent covered change')
    git(writer, 'push', 'origin', 'HEAD:main')
    previous = git(ROOT, '--git-dir', str(remote), 'rev-parse', 'main')
    case = {'old_content_key': guard.content_key(builder), 'remote_before': previous}
    case['land'] = run(builder, 'stale-proof-land', ['python3', LOOP, 'land', '--worktree', '.'])
    case['new_content_key'] = guard.content_key(builder)
    case['remote_after'] = git(ROOT, '--git-dir', str(remote), 'rev-parse', 'main')
    log = (OUT / 'stale-proof-land.log').read_text()
    if (case['old_content_key'] == case['new_content_key'] or previous != case['remote_after']
            or '"result": "FAILED"' not in log or 'no exact-baseline proof' not in log):
        raise ValueError('stale proof incorrectly counted after rebase')
    report['cases']['stale-proof'] = case
    report['passed'] = True
    guard.save(OUT / 'report.json', report)
    print('PASS all four land refusals; local remote main unchanged after each refusal')


if __name__ == '__main__':
    main()
