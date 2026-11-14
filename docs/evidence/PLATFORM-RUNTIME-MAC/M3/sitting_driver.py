"""Hold the GPU through the fixed M3 proof and its checked evidence merge.

The caller wraps this command in agent_loop.py lock gpu. Builder inspection
uses two bounded file handshakes; neither is a human approval request.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--merged-commit', required=True)
parser.add_argument('--root', type=Path, default=Path('tmp/mac-m3'))
args = parser.parse_args()
repo = Path.cwd().resolve()
root = args.root.resolve()
assert root.is_relative_to(repo / 'tmp'), root
sitting = root / 'sitting'
sitting.mkdir(parents=True, exist_ok=True)
loop = '/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.2.2/skills/orchestration/build/scripts/agent_loop.py'
deadline = time.monotonic() + 7200
temporary = sitting / 'system-tmp'
temporary.mkdir(exist_ok=True)
environment = dict(os.environ, TMPDIR=str(temporary) + '/')


def save(name, value):
    destination = sitting / name
    pending = destination.with_suffix(destination.suffix + '.pending')
    pending.write_text(json.dumps(value, indent=2) + '\n')
    pending.replace(destination)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def git(*arguments):
    return subprocess.check_output(['git', *arguments], text=True).strip()


def source_match():
    build = json.loads((root / 'build.json').read_text())
    names = subprocess.check_output(['git', 'ls-files', '-z', 'crates', 'examples',
                                    'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo']).decode().split('\0')
    actual = {name: digest(repo / name) for name in names if name and (repo / name).is_file()}
    if actual != build['source_files'] or digest(root / 'v1-game') != build['binary_sha256']:
        raise RuntimeError('merged engine/source set differs from pinned release; rebuild before any trace')
    return build


def run(name, command, seconds=600):
    save('state.json', {'stage': name, 'command': command, 'unix_ns': time.time_ns()})
    started = time.monotonic()
    allowed = min(seconds, deadline - started)
    if allowed <= 0:
        raise RuntimeError('M3 sitting deadline expired')
    with (sitting / (name + '.log')).open('wb') as stream:
        process = subprocess.Popen(command, cwd=repo, env=environment, stdout=stream,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            code = process.wait(timeout=allowed)
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=3)
    save(name + '.json', {'command': command, 'exit': code,
                         'seconds': time.monotonic() - started, 'revision': git('rev-parse', 'HEAD')})
    if code:
        raise RuntimeError(f'{name} failed; inspect {sitting / (name + ".log")}')


def retain_native_selections(directory):
    manifest = json.loads((directory / 'window-manifest.json').read_text())
    records = manifest['records']
    if not manifest['continuous'] or not records:
        raise RuntimeError('cannot reclaim an incomplete continuous sequence')
    initial = max((row for row in records if row['accepted_yaw_degrees'] <= 0), key=lambda row: row['submit_ns'])
    chosen = [initial, *(min(records, key=lambda row: abs(row['accepted_yaw_degrees'] - yaw))
                         for yaw in (9, 18, 27)), records[-1]]
    selected = directory / 'native-selections'
    selected.mkdir(exist_ok=True)
    for row in chosen:
        name = f"{row['submission_id']:06}.png"
        shutil.copy2(directory / 'png' / name, selected / name)
    (selected / 'selected.json').write_text(json.dumps(chosen, indent=2) + '\n')
    source = directory / 'frames.zst'
    expanded = list((directory / 'png').iterdir())
    if len(expanded) != len(records) or any(not p.is_file() or p.suffix != '.png' for p in expanded):
        raise RuntimeError('unexpected PNG expansion contents; preserve for inspection')
    receipt = {'source': str(source), 'sha256': digest(source), 'verified_originals': len(records),
               'verification': 'package_mac_m2_window independently decoded every original hash before playback',
               'reclaimed_png_bytes': sum(p.stat().st_size for p in expanded), 'native_selections': len(chosen)}
    (directory / 'source-archive.json').write_text(json.dumps(receipt, indent=2) + '\n')
    shutil.rmtree(directory / 'png')


def wait_for(name, stage):
    save('state.json', {'stage': stage, 'unix_ns': time.time_ns()})
    until = min(time.monotonic() + 900, deadline)
    while time.monotonic() < until:
        path = sitting / name
        if path.exists():
            return json.loads(path.read_text())
        time.sleep(.5)
    raise RuntimeError(f'bounded builder inspection wait expired: {name}')


keep_awake = None
try:
    if git('status', '--porcelain', '--untracked-files=all'):
        raise RuntimeError('commit the sitting helper before starting qualification')
    run('fetch', ['git', 'fetch', 'origin'], 60)
    run('merged-ancestry', ['git', 'merge-base', '--is-ancestor', args.merged_commit, 'origin/main'], 10)
    covered = json.loads((repo / '.config/exact-baselines.json').read_text())['covered_paths']
    for reference, label in ((args.merged_commit, 'source-merge'), ('origin/main', 'current-main')):
        run(label, ['git', 'diff', '--exit-code', reference, 'HEAD', '--', *covered], 10)
    build = source_match()
    save('source-audit.json', {'merged_commit': args.merged_commit, 'qualification_head': git('rev-parse', 'HEAD'),
                              'origin_main': git('rev-parse', 'origin/main'),
                              'binary_sha256': build['binary_sha256'], 'all_source_files': len(build['source_files'])})
    keep_awake = subprocess.Popen(['/usr/bin/caffeinate', '-d', '-i', '-u', '-t', '7200'])
    time.sleep(.5)
    sys.path.insert(0, str(repo / 'scripts'))
    from qualify_mac_m2 import preflight
    panel = sitting / 'panel'
    panel.mkdir(exist_ok=True)
    preflight(panel)
    playback = str(repo / 'docs/evidence/PLATFORM-RUNTIME-MAC/M3/playback.mjs')
    run('motion-playback', ['node', playback, str(root / 'motion/session/preview-1x.mp4'),
                            str(sitting / 'playback-motion')], 90)
    common = ['python3', 'scripts/qualify_mac_m2.py', '--binary', str(root / 'v1-game'),
              '--store', str(root / 'valley')]
    window = root / 'window'
    run('window', [*common, '--output', str(window), '--window-only'], 300)
    for name in ('grid-replacement', 'valley-replacement'):
        run('package-' + name, ['python3', 'scripts/package_mac_m2_window.py', '--output',
                               str(window / name), '--verify-only'])
    run('package-window-pan', ['python3', 'scripts/package_mac_m2_window.py', '--output',
                               str(window / 'on-untraced-current')])
    run('window-pan-playback', ['node', playback, str(window / 'on-untraced-current/preview-1x.mp4'),
                               str(sitting / 'playback-window-pan')], 90)
    retain_native_selections(window / 'on-untraced-current')
    acceptance = wait_for('window-acceptance.json', 'await-window-builder-inspection')
    if acceptance.get('M2.3') != 'pass' or acceptance.get('binary_sha256') != build['binary_sha256']:
        raise RuntimeError('same-build window inspection has not passed')
    presented = root / 'presented'
    for name in ('off-traced', 'on-untraced', 'on-traced'):
        run(name, [*common, '--output', str(presented), '--identity', str(window / 'identity.json'),
                   '--post-apple-tile-m3', '--presented-minimal', '--presented-deferred',
                   '--presented-arm', name], 360)
        if name != 'on-untraced':
            run('witness-' + name, ['python3', 'scripts/mac_m2_presented.py', '--input',
                                    str(presented / name / 'presented.xml'), '--arm', str(presented / name),
                                    '--output', str(presented / name / 'witness.json')], 60)
        if name != 'off-traced':
            run('package-' + name, ['python3', 'scripts/package_mac_m2_window.py', '--output', str(presented / name)])
            run('playback-' + name, ['node', playback, str(presented / name / 'preview-1x.mp4'),
                                     str(sitting / ('playback-' + name))], 90)
            retain_native_selections(presented / name)
    run('analysis', ['python3', 'docs/evidence/render/mac-m2-20261113/analyse.py', '--motion',
                    str(root / 'motion/session'), '--window', str(window), '--presented', str(presented),
                    '--output', str(root / 'analysis.json')], 60)
    run('cadence', ['python3', 'docs/evidence/render/mac-m2-20261113/cadence.py', '--root', str(presented),
                   '--output', str(root / 'cadence.json')], 60)
    final = wait_for('ready-to-merge.json', 'await-final-builder-inspection-and-evidence-commit')
    if final.get('commit') != git('rev-parse', 'HEAD') or git('status', '--porcelain', '--untracked-files=all'):
        raise RuntimeError('final evidence commit is missing or checkout changed')
    source_match()
    run('final-check', ['python3', loop, 'check', '--module', 'PLATFORM-RUNTIME-MAC',
                        '--milestone', 'M3', '--worktree', '.', '--timeout', '3600'], 3660)
    run('final-merge', ['python3', loop, 'merge', '--worktree', '.'], 3600)
    source_match()
    save('complete.json', {'commit': git('rev-parse', 'HEAD'), 'source_unchanged': True})
    save('state.json', {'stage': 'complete', 'unix_ns': time.time_ns()})
except Exception as error:
    save('failure.json', {'error': str(error), 'unix_ns': time.time_ns()})
    raise
finally:
    if keep_awake is not None:
        keep_awake.terminate()
        keep_awake.wait(timeout=5)
