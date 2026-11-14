#!/usr/bin/env python3
"""Capture M3's five on/off motion pairs. Run under agent_loop.py lock gpu.

Build the native and web products first. Full lossless source archives remain;
verified PNG derivatives are reduced to selected native-size frames after encoding.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compact_pngs(session, target):
    manifest = json.loads((session/'manifest.json').read_text())
    records = manifest['records']
    selected = {0,len(records)//4,len(records)//2,3*len(records)//4,len(records)-1}
    if target == 'mac-metal':
        images = json.loads((session/'runtime.json').read_text())['images']
        spawn = next((i for i,row in enumerate(images) if row.get('source_tick',row.get('tick',0)) >= 60),None)
        if spawn is not None:
            selected.update(range(max(0,spawn-2),min(len(records),spawn+4)))
    keep = {f'{records[i]["submission_id"]:06}.png' for i in selected}
    result = {'full_source_archive_retained':True,'selected_pngs':{},'preview_sha256':sha(session/'preview-1x.mp4')}
    for path in (session/'png').glob('*.png'):
        if path.name in keep:
            result['selected_pngs'][path.name] = sha(path)
        else:
            path.unlink()
    (session/'selected.json').write_text(json.dumps(result,indent=2)+'\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target',required=True,choices=['mac-metal','chrome-webgpu'])
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--store',required=True,type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(ROOT/'tmp') or output.exists():
        raise ValueError('use a new output directory under this repository tmp/')
    output.mkdir(parents=True)
    report = {'commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
              'target':args.target,'mode':'realtime','load_start':os.getloadavg(),'arms':[],
              'comparison':'capture on in both arms; TAAU on/off; native 1920x1080, Chrome route extent',
              'complete':False}
    cases = {'pan':'reconstruction-pan','reveal':'running','spawn':'running',
             'stars':'reconstruction-stars','scrub':'reconstruction-scrub'}
    try:
        for case,native_case in cases.items():
            for mode in ('taau','off'):
                directory = output/f'{case}-{mode}'
                if args.target == 'mac-metal':
                    preset = 'night-upslope' if case == 'stars' else 'noon'
                    command = [sys.executable,'scripts/capture_mac_m2_ungranted.py','--output',str(directory),
                               '--binary','target/release/v1-game','--store',str(args.store.resolve()),
                               '--case',native_case,'--params',f'examples/v1-game/params/showcase-{preset}.params.json',
                               '--recipe','portable','--overlay','reconstruction','--reconstruction',mode]
                    session = directory/'session'
                    package = [sys.executable,'scripts/capture_v1_runtime.py','--output',str(session),'--finalize']
                else:
                    command = ['node','scripts/capture_v1_browser.mjs','--output',str(directory),'--mode','realtime',
                               '--cook','target/release/thrum-cook','--witness',case,'--reconstruction',mode]
                    session = directory
                    package = [sys.executable,'scripts/package_v1_browser.py','--output',str(session)]
                row = {'case':case,'reconstruction':mode,'command':command,'package':package,'load':os.getloadavg()}
                report['arms'].append(row)
                with (output/f'{case}-{mode}.log').open('w') as log:
                    subprocess.run(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=300)
                    subprocess.run(package,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=300)
                manifest = json.loads((session/'manifest.json').read_text())
                if not manifest['complete']:
                    raise ValueError(f'{case}-{mode}: incomplete target capture')
                compact_pngs(session,args.target)
                shutil.rmtree(directory/'swift-cache')
                row.update(complete=True,manifest_sha256=sha(session/'manifest.json'),records=len(manifest['records']))
        report['complete'] = True
    finally:
        report['load_end'] = os.getloadavg()
        (output/'campaign.json').write_text(json.dumps(report,indent=2)+'\n')


if __name__ == '__main__':
    main()
