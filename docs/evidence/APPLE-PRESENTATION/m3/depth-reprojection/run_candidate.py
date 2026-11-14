#!/usr/bin/env python3
"""Qualify one clean M3 depth candidate. The caller must hold the gpu lock."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[5]


def git(*args):
    return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args()
    output=(ROOT/args.output).resolve()
    if not output.is_relative_to(ROOT/'tmp') or output.exists():
        raise ValueError('use a new output directory under repository tmp/')
    if git('status','--porcelain','--untracked-files=all'):
        raise ValueError('candidate requires a clean committed tree')
    output.mkdir(parents=True)
    store=output/'store'
    report={'complete':False,'commit':git('rev-parse','HEAD'),
            'scope':'M3 experimental depth candidate; GPU serialized by caller; no cadence or photon claim',
            'load_start':os.getloadavg(),'phases':[]}
    def save():
        (output/'run.json').write_text(json.dumps(report,indent=2)+'\n')
    phases=[
        ('metal-depth-probe',['rustup','run','1.99.1','cargo','test','--locked','--offline','-p','thrum-render','taau_','--','--nocapture'],300),
        ('reconstruction',[sys.executable,'scripts/run_v1_showcase.py','reconstruction','--target','mac-metal'],900),
        ('web-build',['sh','examples/v1-game/web/build.sh'],600),
        ('cook',['target/release/thrum-cook','cook','examples/v1-game/assets/showcase',str(store)],120),
        ('exact-baselines',[sys.executable,'scripts/exact_baselines.py','verify'],2400),
        ('mac-motion',[sys.executable,'scripts/apple_presentation_m3_motion.py','--target','mac-metal','--output',str(output/'mac-motion'),'--store',str(store)],1800),
        ('chrome-motion',[sys.executable,'scripts/apple_presentation_m3_motion.py','--target','chrome-webgpu','--output',str(output/'chrome-motion'),'--store',str(store)],1800),
        ('route-timing',[sys.executable,'scripts/apple_presentation_m3_timing.py','--output',str(output/'route-timing'),'--store',str(store)],300),
        ('moving',[sys.executable,'scripts/run_v1_showcase.py','moving'],900),
    ]
    save()
    try:
        for name,command,timeout in phases:
            phase={'name':name,'command':command,'state':'running','load_start':os.getloadavg()}
            report['phases'].append(phase)
            save()
            print('RUN',name,flush=True)
            started=time.monotonic()
            with (output/(name+'.log')).open('w') as log:
                result=subprocess.run(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,timeout=timeout,check=False)
            phase.update(exit=result.returncode,seconds=time.monotonic()-started,load_end=os.getloadavg(),state='complete')
            save()
            if result.returncode:
                raise RuntimeError(f'{name} failed with exit {result.returncode}; see {output/(name+".log")}')
        if git('rev-parse','HEAD') != report['commit'] or git('status','--porcelain','--untracked-files=all'):
            raise RuntimeError('source changed during candidate qualification')
        report['complete']=True
    except Exception as error:
        report['error']=str(error)
        raise
    finally:
        report['load_end']=os.getloadavg()
        save()


if __name__=='__main__':
    main()
