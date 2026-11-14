#!/usr/bin/env python3
"""Paired portable moving-route timing at M3's 640x360 extent. Run under the GPU lock."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
from statistics import median
import subprocess

from run_v1_showcase import MOVING_ROUTE, REPO, identities, moving_native, write_json


def summary(values):
    values = sorted(values)
    return {'frames':len(values),'p50_ns':median(values),'p95_ns':values[math.ceil(.95*len(values))-1]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--store',required=True,type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(REPO/'tmp') or output.exists():
        raise ValueError('use a new output directory under repository tmp/')
    output.mkdir(parents=True)
    route = json.loads(MOVING_ROUTE.read_text())
    route['render_extent'] = [640,360]
    route_path = output/'route.json'
    write_json(route_path,route)
    report = {'complete':False,'commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip(),
              'builds':identities(web=False,built=False),'extent':[640,360],
              'route_sha256':hashlib.sha256(route_path.read_bytes()).hexdigest(),
              'canonical_route_sha256':hashlib.sha256(MOVING_ROUTE.read_bytes()).hexdigest(),
              'route_difference':'render_extent only: 640x360 instead of canonical 1280x720',
              'target':'mac-metal','order':['taau','off'],'arms':{},
              'scope':'ordinary fixed-time moving game route, one world tick per submission, alpha 0.5; capture only at route checkpoints in both arms; registered GPU jobs serialized; shared CPU/host load',
              'design_1080p_estimate_ms':[.2,.5]}
    identities_by_arm = []
    try:
        os.environ['THRUM_RECIPE'] = 'portable'
        for mode in report['order']:
            os.environ['THRUM_RECONSTRUCTION'] = mode
            load = os.getloadavg()
            header,samples = moving_native(output,mode,args.store.resolve(),route_path)
            if header['render_extent'] != [640,360] or header['recipe'] != 'Ok(Portable)' or header['reconstruction'] != ('Taau' if mode=='taau' else 'Off'):
                raise ValueError(f'{mode}: prepared route configuration differs')
            rows = list(samples.values())
            expected = route['keys'][-1]['tick']+1
            if len(rows) != expected or any(row['gpu_pass_ns'] is None for row in rows):
                raise ValueError(f'{mode}: missing route frames/timestamps')
            passes = [row['gpu_pass_ns'] for row in rows]
            resolve = [values['portable TAAU'] for values in passes if values.get('portable TAAU') is not None]
            if len(resolve) != (len(rows) if mode=='taau' else 0) or (mode=='off' and any('portable TAAU' in values for values in passes)):
                raise ValueError(f'{mode}: executed resolve differs')
            fields = ['tick','route_tick','alpha_bits','committed_solar_bits','rendered_solar_bits','eye','yaw','pitch']
            identities_by_arm.append([{key:row[key] for key in fields} for row in rows])
            scene = [sum(value for name,value in values.items() if value is not None and name!='V1 surface present') for values in passes]
            report['arms'][mode] = {'load_start':load,'load_end':os.getloadavg(),'header':header,
                                    'scene':summary(scene),'resolve':summary(resolve) if resolve else None}
        if identities_by_arm[0] != identities_by_arm[1]:
            raise ValueError('paired route world/camera/solar/alpha inputs differ')
        report['paired_inputs_equal'] = True
        # This is explicitly a pixel-ratio heuristic, never a measured 1080p value.
        heuristic = report['arms']['taau']['resolve']['p50_ns']*9/1e6
        report['pixel_ratio_1080p_heuristic_ms'] = heuristic
        report['tripwire_only_outside_estimate'] = not .2 <= heuristic <= .5
        report['complete'] = True
    finally:
        write_json(output/'run.json',report)


if __name__ == '__main__':
    main()
