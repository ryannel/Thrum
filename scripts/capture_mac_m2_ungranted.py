#!/usr/bin/env python3
"""Launch the existing T2 Mac pan from a fresh, ungranted local app identity.

Run under the GPU lock. LaunchServices supplies TCC attribution; neither the
launcher nor driver requests permission or changes console lock.
"""
import argparse
import json
import plistlib
from pathlib import Path
import subprocess
import sys
import uuid

from capture_v1_runtime import ROOT, digest, validate_permission_attribution

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/v1-game')
parser.add_argument('--store', type=Path, default=ROOT / 'tmp/mac-m2/valley')
parser.add_argument('--case', default='pan', choices=['pan','running','reconstruction-pan','reconstruction-stars','reconstruction-scrub'])
parser.add_argument('--params', type=Path, default=ROOT / 'examples/v1-game/params/showcase-noon.params.json')
parser.add_argument('--reconstruction', choices=['off','taau'])
parser.add_argument('--overlay', choices=['off','proof','motion','reconstruction'])
parser.add_argument('--recipe', choices=['portable','apple-tile'])
args = parser.parse_args()
output = args.output.resolve()
output.mkdir(parents=True, exist_ok=False)
bundle = output / 'ThrumMacM2.app'
contents = bundle / 'Contents'
(contents / 'MacOS').mkdir(parents=True)
(contents / 'Info.plist').write_bytes(plistlib.dumps({
    'CFBundleExecutable': 'Launcher', 'CFBundleIdentifier': 'org.thrum.mac-m2.capture.' + uuid.uuid4().hex,
    'CFBundleName': 'Thrum Mac M2 capture', 'CFBundlePackageType': 'APPL', 'LSUIElement': True}))
launcher = contents / 'MacOS/Launcher'
subprocess.run(['/usr/bin/swiftc', '-module-cache-path', str(output / 'swift-cache'),
                str(ROOT / 'scripts/mac_m2_capture_launcher.swift'), '-o', str(launcher)], check=True, timeout=60)
subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(bundle)], check=True, timeout=30)
command = ['/usr/bin/open', '-n', '-W', str(bundle), '--args', str(output), sys.executable, str(ROOT),
           str(ROOT / 'scripts/capture_v1_runtime.py'), '--binary', str(args.binary.resolve()),
           '--store', str(args.store.resolve()), '--output', str(output / 'session'),
           '--case', args.case, '--params', str(args.params.resolve()), '--mode', 'realtime', '--defer']
for option in ['reconstruction','overlay','recipe']:
    if getattr(args,option) is not None:
        command += ['--'+option,getattr(args,option)]
(output / 'invocation.json').write_text(json.dumps({'command': command, 'launcher_sha256': digest(launcher),
    'launcher_source_sha256': digest(ROOT / 'scripts/mac_m2_capture_launcher.swift')}, indent=2) + '\n')
subprocess.run(command, cwd=ROOT, check=True, timeout=180)
report = json.loads((output / 'launcher.json').read_text())
if report.get('error') or report.get('capture_exit_code') != 0:
    log = output / 'capture.log'
    if log.exists():
        print(log.read_text()[-5000:], file=sys.stderr)
    raise SystemExit(report.get('error') or f"capture driver exited {report.get('capture_exit_code')}")
manifest = json.loads((output / 'session/manifest.json').read_text())
identity = plistlib.loads((contents / 'Info.plist').read_bytes())['CFBundleIdentifier']
validate_permission_attribution(report, manifest, identity)
print('Denied attributed launcher/driver preflights verified. Console observations retained. Finalize and inspect the session before qualifying target motion.')
