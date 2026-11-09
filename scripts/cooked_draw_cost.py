#!/usr/bin/env python3
"""Paired production path costs on this Mac; no path-promotion or simulator result."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess

from cooked_draw_fixture import environment, sources, cook, placements
from cooked_draw_parity import ROOT, digest
from t2_load import LoadRecord


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    os.chdir(ROOT)
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    env=environment()
    subprocess.run(['cargo','build','--locked','--offline','--release','-p','v1-game','--features','draw-probe','--bin','cooked_draw_probe'],env=env,check=True)
    binary=ROOT/'target/release/cooked_draw_probe'
    sources(out/'source',env);cook(out/'source',out/'store',env)
    report={'target':'mac-metal','profile':'release','scope':'paired descriptive figures; no speed or promotion claim; GPU lock held; fixed submitted 1920x1080 valley noon; waits and image export outside recorded encode/GPU intervals',
            'commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
            'diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD'])).hexdigest(),
            'binary_sha256':digest(binary),'content_hashes':{str(p.relative_to(out/'store')):digest(p) for p in (out/'store').rglob('*') if p.is_file()},'runs':[],'pass':False}
    load=LoadRecord(out)
    try:
        for pair,order in enumerate([['I','D'],['D','I'],['I','D'],['D','I']]):
            for position,arm in enumerate(order):
                folder=out/f'{pair}-{position}-{arm}'
                run_env=dict(env,THRUM_SCENE='valley',THRUM_ASSET_STORE=str(out/'store'),THRUM_STARS='off',
                    COOKED_DRAW_OUTPUT=str(folder),COOKED_DRAW_MASK_INDIRECT=str(int(arm=='D')),
                    COOKED_DRAW_MODE='production',COOKED_DRAW_YAW='0',COOKED_DRAW_SOLAR='12',
                    COOKED_DRAW_MESHES=json.dumps(placements('valley')),COOKED_DRAW_SAMPLES='128')
                before=list(os.getloadavg())
                subprocess.run([str(binary)],env=run_env,check=True)
                data=json.loads((folder/'report.json').read_text())
                if data['path']!=f'Some({"Indirect" if arm=="I" else "Direct"})' or data['diagnostic_mode']!='production':
                    raise RuntimeError('cost used wrong encoder')
                samples=data['timing_samples']
                if len(samples)!=128 or any(s['forward_ns'] is None for s in samples):
                    raise RuntimeError('incomplete GPU/CPU cost samples')
                def summary(key):
                    values=[s[key] for s in samples if s[key] is not None]
                    return {'present':len(values),'median_ns':statistics.median(values) if values else None,'p95_ns':sorted(values)[int(.95*(len(values)-1))] if values else None}
                logical=int(subprocess.check_output(['sysctl','-n','hw.logicalcpu'],text=True))
                load_class='quiet' if before[0]<logical else 'busy' if before[0]<=4*logical else 'saturated'
                report['runs'].append({'pair':pair,'order':order,'position':position,'arm':arm,'load_before':before,'load_after':list(os.getloadavg()),'logical_cpus':logical,'load_class':load_class,'warmup_drawn_frames':16,'samples':samples,'summary':{key:summary(key) for key in ['forward_ns','contribution_ns','encode_ns']},'path':data['path'],'adapter_flags':data['adapter_flags'],'flags':data['flags'],'commands':data['encoded_commands'],'resources':data['resource_observation']})
        report['pass']=True
    finally:
        report['load']=load.finish()
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':main()
