"""Attributed, read-only permission proof around the actual simulator harness."""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import uuid

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


def launch(arguments, output, driver=None):
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
        subprocess.run(['/usr/bin/swiftc', '-module-cache-path', str(support / 'swift-cache'),
                        str(source), '-o', str(executable)], check=True, timeout=120)
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(bundle)], check=True, timeout=30)
        command = ['/usr/bin/open', '-n', '-W', str(bundle), '--args', str(support), sys.executable,
                   str(ROOT), os.environ['PATH'], str(driver or ROOT / 'scripts/cooked_draw_ios_sim.py'), *arguments]
        result = subprocess.run(command, cwd=ROOT, check=False, timeout=1800)
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
        if result.returncode:
            raise RuntimeError(f'LaunchServices launcher exited {result.returncode}')
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
