"""Attributed, read-only permission proof around the actual simulator harness."""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import signal
import subprocess
import sys
import time
import uuid

import simulator_lifecycle as simulator

ROOT = Path(__file__).resolve().parents[1]
REPORT_ENV = 'THRUM_COOKED_DRAW_LAUNCHER_REPORT'


def validate_preflights(launcher, route, identity):
    """Require denied endpoints from the app that actually parented this harness."""
    if (launcher.get('error') or type(launcher.get('driver_exit_code')) is not int
            or launcher['driver_exit_code'] != 0):
        raise RuntimeError(launcher.get('error') or route.get('failure')
                           or f"simulator harness exited {launcher.get('driver_exit_code')}")
    if any(launcher.get(key) is not False for key in ['screen_recording_before', 'screen_recording_after']):
        raise RuntimeError('denied before/after Screen Recording preflights required')
    if launcher.get('bundle_identifier') != identity:
        raise RuntimeError('Screen Recording permission identity mismatch')
    if launcher.get('preflight_api') != 'CGPreflightScreenCaptureAccess' or not launcher.get('launcher_executable'):
        raise RuntimeError('Screen Recording preflight API or executable identity missing')
    if (any(type(launcher.get(key)) is not int or launcher[key] <= 0 for key in ['launcher_pid', 'driver_pid'])
            or launcher['launcher_pid'] != route.get('launcher_pid')
            or launcher['driver_pid'] != route.get('harness_pid')):
        raise RuntimeError('permission preflight is not attributed to the route launcher')
    if route.get('route_pass') is not True:
        raise RuntimeError('simulator route did not pass')


def attributed_device():
    """Stay inert until the supervisor has the receipt for our process."""
    if os.getpgrp() != os.getpid():
        os.setpgid(0, 0)
    permit = Path(os.environ[REPORT_ENV]).with_name('driver-go')
    deadline = time.monotonic() + 15
    while not permit.is_file():
        if time.monotonic() >= deadline:
            raise TimeoutError('Missing supervisor permit for the attributed driver')
        time.sleep(0.05)
    if permit.read_text() != str(os.getpid()):
        raise RuntimeError('Supervisor permit belongs to another driver')
    return simulator.worker_device(attributed=True)


def stop_attributed(process, receipt, executable):
    # open -W is in our worker group. LaunchServices starts the app outside
    # that group, and the driver has its own group for XCTest and capture.
    simulator.stop_worker(process)
    row = receipt()
    if row.get('launcher_executable') != str(executable):
        return
    launcher, driver = (row.get(key) for key in ('launcher_pid', 'driver_pid'))
    for pid, expected in ((launcher, str(executable)), (driver, sys.executable)):
        if not isinstance(pid, int):
            continue
        current = subprocess.run(['ps', '-p', str(pid), '-o', 'command='],
                                 capture_output=True, text=True, timeout=0.3)
        if expected not in current.stdout:
            continue
        try:
            if pid == driver and os.getpgid(pid) == pid:
                os.killpg(pid, signal.SIGKILL)
            else:
                os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def run_attributed(turn, command, support, executable):
    def receipt():
        try:
            return json.loads((support / 'launcher.json').read_text())
        except FileNotFoundError:
            return {}

    with subprocess.Popen(command, cwd=ROOT, start_new_session=True) as process:
        try:
            while process.poll() is None:
                turn.check()
                row = receipt()
                if (row.get('launcher_executable') == str(executable)
                        and row.get('screen_recording_before') is False):
                    (support / 'launcher-go').touch(exist_ok=True)
                    if driver := row.get('driver_pid'):
                        permit = support / 'driver-go'
                        if not permit.exists():
                            staging = permit.with_suffix('.partial')
                            staging.write_text(str(driver))
                            staging.replace(permit)
                time.sleep(0.05)
            return process.returncode
        except BaseException:
            stop_attributed(process, receipt, executable)
            raise


def launch(arguments, output, driver=None, *, requested=None):
    holder, _ = simulator.admission()
    if holder is None:
        simulator.exec_with_admission([sys.executable, sys.argv[0], *arguments])

    output = output.resolve()
    if output.exists():
        raise FileExistsError(output)
    support = output.with_name(output.name + '-launcher')
    support.mkdir(parents=True, exist_ok=False)
    bundle = support / 'ThrumCookedDraw.app'
    contents = bundle / 'Contents'
    (contents / 'MacOS').mkdir(parents=True)
    identity = 'org.thrum.cooked-draw.qualification.' + uuid.uuid4().hex
    (contents / 'Info.plist').write_bytes(plistlib.dumps({
        'CFBundleExecutable': 'Launcher', 'CFBundleIdentifier': identity,
        'CFBundleName': 'Thrum Cooked Draw qualification', 'CFBundlePackageType': 'APPL', 'LSUIElement': True}))
    executable = contents / 'MacOS/Launcher'
    source = ROOT / 'scripts/cooked_draw_sim_launcher.swift'
    report = {'target': 'ios-sim', 'pass': False, 'launcher_directory': str(support)}
    try:
        with simulator.supervise(requested=requested, fresh=True) as turn:
            turn.run(['/usr/bin/swiftc', '-module-cache-path', support / 'swift-cache',
                      source, '-o', executable])
            turn.run(['/usr/bin/codesign', '--force', '--sign', '-', bundle])
            command = ['/usr/bin/open', '-n', '-W', str(bundle), '--args', str(support), sys.executable,
                       str(ROOT), os.environ['PATH'], turn.udid,
                       str(driver or ROOT / 'scripts/cooked_draw_ios_sim.py'), *arguments]
            result = run_attributed(turn, command, support, executable)
        if (output / 'report.json').exists():
            report.update(json.loads((output / 'report.json').read_text()))
        launcher = json.loads((support / 'launcher.json').read_text())
        report['permission_preflight'] = dict(launcher,
            expected_bundle_identifier=identity,
            launcher_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),
            launcher_source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
            launch_command=command,
            attribution='LaunchServices app is the direct parent of the Python harness that launches simctl and the route')
        validate_preflights(launcher, report, identity)
        if result:
            raise RuntimeError(f'LaunchServices launcher exited {result}')
        report['pass'] = True
    except BaseException as error:
        report['pass'] = False
        report['failure'] = str(error)
        log = support / 'harness.log'
        if log.exists():
            print(log.read_text()[-5000:], file=sys.stderr)
        raise
    finally:
        output.mkdir(parents=True, exist_ok=True)
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
