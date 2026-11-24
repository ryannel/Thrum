"""Induce a scheduling gap in an owned host process and verify its lifecycle replies."""

import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time


def wait_for(label, observe, predicate, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        value = observe()
        if predicate(value):
            return value
        time.sleep(0.05)
    raise RuntimeError(f'timed out waiting for {label}: {value}')


def force_scheduling_gap(pid):
    os.kill(pid, signal.SIGSTOP)
    try:
        # Twelve 60 Hz ticks would already exceed the local queue.
        time.sleep(0.6)
    finally:
        os.kill(pid, signal.SIGCONT)


def check_mac_overload(folder, preset):
    output = folder / 'overload'
    output.mkdir()
    markers = output / 'pixels'
    binary = folder / 'target/release/v1-game'
    log = output / 'game.log'
    env = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(folder / 'store'),
               THRUM_PARAMS=str(preset), THRUM_V1_FRAMES='10000', THRUM_REPORT='1',
               THRUM_LIVE_WITNESS=str(markers), THRUM_LIVE_DESTROY='0')
    with log.open('wb') as stream, subprocess.Popen(
        [str(binary)], env=env, stdout=stream, stderr=subprocess.STDOUT
    ) as game:
        def events():
            if game.poll() is not None:
                raise RuntimeError(f'overload witness exited {game.returncode}')
            return re.findall(
                r'm1 app pause-ack request=(Pause|Resume) endpoint=SnapshotId '
                r'\{ epoch: WorldEpoch\((\d+)\), tick: (\d+) \}', log.read_text(errors='replace'))

        try:
            wait_for('initial Mac Pause', events, lambda rows: rows and rows[-1][0] == 'Pause')
            wait_for('first Mac present', log.read_text,
                     lambda output: 'm1 app presented generation=1' in output)
            (markers / 'press-p').touch(exist_ok=False)
            before = wait_for('explicit Mac Resume', events, lambda rows: rows[-1][0] == 'Resume')
            force_scheduling_gap(game.pid)
            after = wait_for('Mac overload Pause and Resume', events, lambda rows: len(rows) >= len(before) + 2)
            pause, resume = after[len(before):len(before) + 2]
            if pause[0] != 'Pause' or resume[0] != 'Resume' or pause[1:] != resume[1:]:
                raise RuntimeError(f'Mac overload changed the drained endpoint: {pause}, {resume}')
            if pause[1] != before[-1][1]:
                raise RuntimeError('Mac overload replaced the world')
            presented = log.read_text().count('m1 queue present')
            wait_for('Mac presents after overload', log.read_text,
                     lambda output: output.count('m1 queue present') >= presented + 4)
            (markers / 'finish').touch(exist_ok=False)
            if game.wait(timeout=30) != 0:
                raise RuntimeError(f'Mac overload witness exited {game.returncode}')
        finally:
            if game.poll() is None:
                game.kill()
    match = re.search(r'committed_ticks=(\d+); .*world_overloads=(\d+); overload_refused_ticks=(\d+)',
                      log.read_text())
    if match is None or int(match[1]) <= int(resume[2]) or int(match[2]) < 1 or int(match[3]) < 1:
        raise RuntimeError('Mac overload did not report any refused ticks')
    record = dict(pid=game.pid, pause=pause, resume=resume, final_tick=int(match[1]),
                  overloads=int(match[2]), refused_ticks=int(match[3]),
                  binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest())
    (output / 'result.json').write_text(json.dumps(record, indent=2) + '\n')
    print(f'Mac window overload recovered at epoch {resume[1]}, tick {resume[2]}', flush=True)


def check_ios_overload(folder, app, device):
    def simctl(*args, **kwargs):
        return subprocess.check_output(['xcrun', 'simctl', *args], timeout=90,
                                       text=True, **kwargs).strip()

    # The route just used this boot. Install the overload witness in a fresh
    # app container while preserving the device identity.
    subprocess.run(['xcrun', 'simctl', 'terminate', device, 'org.thrum.v1'],
                   capture_output=True, timeout=15)
    simctl('uninstall', device, 'org.thrum.v1')
    simctl('install', device, str(app))
    container = Path(simctl('get_app_container', device, 'org.thrum.v1', 'data'))
    launch = simctl('launch', device, 'org.thrum.v1',
                    env=dict(os.environ, SIMCTL_CHILD_THRUM_V1_ROUTE='1'))
    pid = int(re.fullmatch(r'org\.thrum\.v1: (\d+)', launch)[1])
    state_file = container / 'tmp/thrum-v1-ios-state.txt'

    def state():
        if not state_file.exists():
            return {}
        fields = dict(line.split('=', 1) for line in state_file.read_text().splitlines() if '=' in line)
        if 'fault' in fields:
            raise RuntimeError(fields['fault'])
        return fields

    def number(fields, key):
        return int(fields.get(key, '0'))

    before = wait_for('running iPhone simulator game', state, lambda fields:
                      fields.get('state') == 'Active' and number(fields, 'pid') == pid
                      and number(fields, 'presented') > 10 and fields.get('world_paused') == 'false',
                      seconds=90)
    force_scheduling_gap(pid)

    def recovered(fields):
        return (number(fields, 'overloads') > number(before, 'overloads')
                and number(fields, 'pause_replies') > number(before, 'pause_replies')
                and number(fields, 'resume_replies') > number(before, 'resume_replies')
                and fields.get('world_paused') == 'false'
                and number(fields, 'world_tick') > number(fields, 'resume_tick'))

    after = wait_for('iPhone overload, confirmed drain and resumed authority', state, recovered)
    if (after['pause_epoch'], after['pause_tick']) != (after['resume_epoch'], after['resume_tick']):
        raise RuntimeError('iPhone overload changed the drained world endpoint')
    if number(after, 'pid') != pid or number(after, 'refused_ticks') <= number(before, 'refused_ticks'):
        raise RuntimeError('iPhone overload did not recover in the original game with recorded refusals')
    record = dict(device=device, binary_sha256=hashlib.sha256((app / 'ThrumV1').read_bytes()).hexdigest(),
                  before=before, after=after)
    (folder / 'overload.json').write_text(json.dumps(record, indent=2) + '\n')
    print(f'iPhone simulator overload recovered at epoch {after["resume_epoch"]}, '
          f'tick {after["resume_tick"]} (function only)', flush=True)


if __name__ == '__main__':
    import sys
    from simulator_lifecycle import worker_device

    check_ios_overload(Path(sys.argv[1]), Path(sys.argv[2]), worker_device())
