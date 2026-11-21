#!/usr/bin/env python3
"""Cost record for the river solve (RIVERS M1.8).

`record` builds the solver in release, solves the M1.1 test valley at 0.25 m cells on one thread
and on all of the machine's, and writes the working folder's .orchestrate/evidence/RIVERS/river-cost.json
with the wet cells, steps, cell updates and seconds, the commit, the load on the machine and the build.
It compares the cell updates with part 5 of tests/fixtures/rivers/river_proof.py.

`check` fails only when the solve cannot be measured or the committed record, the test data in
tests/fixtures/rivers/river-cost.json, lacks a field; promoting a new take is copying it there. No
figure in the record decides anything; it is a record, not a gate (brief decision 3).
"""
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RECORD = ROOT / 'tests/fixtures/rivers/river-cost.json'
TAKE = ROOT / '.orchestrate/evidence/RIVERS/river-cost.json'
FIELDS = ('wet_cells', 'steps', 'cell_updates', 'seconds')
TOP = ('commit', 'load', 'build', 'runs', 'part5_comparison')
# Part 5: a 350 m stream 4 m wide at 0.25 m cells, 22,400 wet cells and about 24,267 steps, so
# 544 million cell updates for each flow sample.
PART5 = {'wet_cells': 22400, 'steps': 24267, 'cell_updates': 544e6}


def run(*args, **kw):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, capture_output=True, **kw).stdout.strip()


def build():
    run('cargo', 'build', '--locked', '--offline', '--release', '-p', 'thrum-river-model',
        '--example', 'valley_cost')


def measure(threads):
    out = run('cargo', 'run', '--locked', '--offline', '--release', '-q', '-p', 'thrum-river-model',
              '--example', 'valley_cost', '--', '0.25', str(threads))
    row = json.loads(out.splitlines()[-1])
    for f in FIELDS:
        if not (isinstance(row.get(f), (int, float)) and row[f] > 0):
            raise SystemExit(f'river cost: the solve reported no {f}: {row}')
    return row


def machine():
    try:
        cpu = run('sysctl', '-n', 'machdep.cpu.brand_string')
    except (OSError, subprocess.CalledProcessError):
        cpu = platform.processor()
    return {'cpu': cpu, 'system': platform.platform(), 'logical_cpus': os.cpu_count()}


def record():
    build()
    load_before = os.getloadavg()
    runs = [measure(1), measure(0)]
    load_after = os.getloadavg()
    dirty = bool(run('git', 'status', '--porcelain', '--', 'crates', 'Cargo.toml', 'Cargo.lock'))
    one = runs[0]
    per_wet = one['cell_updates'] / one['wet_cells']
    rec = {
        'what': 'A solve of the RIVERS M1.1 test valley (48 m by 14 m, 0.3 m3/s) at 0.25 m cells, '
                'from pools filled to their sills, to the first profile\'s stopping rule.',
        'commit': run('git', 'rev-parse', 'HEAD'),
        'solver_sources_dirty': dirty,
        'build': 'release (cargo --release, opt-level 3)',
        'rustc': run('rustc', '--version'),
        'machine': machine(),
        'load': {'before_1_5_15_min': [round(v, 2) for v in load_before],
                 'after_1_5_15_min': [round(v, 2) for v in load_after],
                 'note': 'Other builders share this Mac, so seconds are noisy; cell updates are exact.'},
        'runs': runs,
        'part5_comparison': {
            'part5': PART5,
            'part5_updates_per_wet_cell': round(PART5['cell_updates'] / PART5['wet_cells']),
            'valley_updates_per_wet_cell': round(per_wet),
            'valley_steps': one['steps'],
            'words': (f"The valley takes {one['steps']} steps and {one['cell_updates'] / 1e6:.1f} million cell "
                      f"updates for {one['wet_cells']} wet cells, {per_wet:.0f} updates a wet cell. Part 5 "
                      f"estimated 24,267 steps for a 350 m stream, {PART5['cell_updates'] / PART5['wet_cells']:.0f} "
                      "updates a wet cell, scaled from a 120 m section; the valley is 40 m long, so it needs "
                      "fewer steps for its signals to cross it, and its updates also count the dry cells "
                      "beside the water that each step works on."),
        },
    }
    TAKE.parent.mkdir(parents=True, exist_ok=True)
    TAKE.write_text(json.dumps(rec, indent=2) + '\n')
    print(json.dumps(rec, indent=2))


def check():
    if not RECORD.exists():
        raise SystemExit(f'river cost: no committed record at {RECORD.relative_to(ROOT)}')
    rec = json.loads(RECORD.read_text())
    missing = [k for k in TOP if k not in rec]
    if 'release' not in str(rec.get('build', '')):
        missing.append('build: release')
    if not rec.get('runs') or any(f not in r for r in rec['runs'] for f in FIELDS):
        missing.append('runs: ' + ', '.join(FIELDS))
    if missing:
        raise SystemExit('river cost: the record lacks ' + '; '.join(missing))
    build()
    row = measure(0)
    print(f"river cost: record at {rec['commit'][:10]} complete; measured now: {row['wet_cells']} wet cells, "
          f"{row['steps']} steps, {row['cell_updates']} cell updates, {row['seconds']:.2f} s")


if __name__ == '__main__':
    command = sys.argv[1] if len(sys.argv) > 1 else ''
    if command == 'record':
        record()
    elif command == 'check':
        check()
    else:
        raise SystemExit('usage: river_cost.py record|check')
