"""Resume only M3's timed-out check and merge; never rerun any recording.

Invoke under agent_loop.py lock gpu. Refuse changed engine/harness inputs.
"""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time

repo = Path.cwd()
root = repo / 'tmp/mac-m3'
out = root / 'finish'
out.mkdir(exist_ok=True)
loop = '/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.2.2/skills/orchestration/build/scripts/agent_loop.py'


def save(name, value):
    path = out / name
    pending = path.with_suffix('.pending')
    pending.write_text(json.dumps(value, indent=2) + '\n')
    pending.replace(path)


def git(*args):
    return subprocess.check_output(['git', *args], text=True).strip()


def source_match():
    build = json.loads((root / 'build.json').read_text())
    names = subprocess.check_output(['git', 'ls-files', '-z', 'crates', 'examples', 'Cargo.toml',
                                    'Cargo.lock', 'rust-toolchain.toml', '.cargo']).decode().split('\0')
    def digest(path):
        with path.open('rb') as stream:
            return hashlib.file_digest(stream, 'sha256').hexdigest()
    actual = {name: digest(repo / name) for name in names if name and (repo / name).is_file()}
    if actual != build['source_files'] or digest(root / 'v1-game') != build['binary_sha256']:
        raise RuntimeError('source differs from qualified assembly; do not repeat traces')


def run(name, command, seconds):
    save('state.json', {'stage': name, 'command': command, 'unix_ns': time.time_ns()})
    started = time.monotonic()
    with (out / (name + '.log')).open('wb') as stream:
        process = subprocess.Popen(command, cwd=repo, stdout=stream, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        try:
            code = process.wait(timeout=seconds)
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=3)
    save(name + '.json', {'exit': code, 'seconds': time.monotonic() - started, 'command': command})
    if code:
        raise RuntimeError(f'{name} failed; inspect {out / (name + ".log")}')


try:
    if git('status', '--porcelain', '--untracked-files=all'):
        raise RuntimeError('commit before finishing M3')
    run('fetch', ['git', 'fetch', 'origin'], 60)
    covered = json.loads((repo / '.config/exact-baselines.json').read_text())['covered_paths']
    run('source-audit', ['git', 'diff', '--exit-code', 'origin/main', 'HEAD', '--', *covered], 30)
    source_match()
    run('check', ['python3', loop, 'check', '--module', 'PLATFORM-RUNTIME-MAC', '--milestone', 'M3',
                  '--worktree', '.', '--timeout', '3600'], 3660)
    run('merge', ['python3', loop, 'merge', '--worktree', '.'], 3660)
    source_match()
    milestone = next(m for m in json.loads((repo / 'docs/plans/PLATFORM-RUNTIME-MAC/DONE.json').read_text())['milestones'] if m['id'] == 'M3')
    if milestone['status'] != 'in_progress' or any(i['builder_result'] != 'pass' for i in milestone['items']):
        raise RuntimeError('M3 builder results/status differ after merge')
    save('complete.json', {'commit': git('rev-parse', 'HEAD'), 'source_unchanged': True})
    save('state.json', {'stage': 'complete', 'unix_ns': time.time_ns()})
except Exception as error:
    save('failure.json', {'error': str(error), 'unix_ns': time.time_ns()})
    raise
