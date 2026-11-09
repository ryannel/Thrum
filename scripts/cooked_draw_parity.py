#!/usr/bin/env python3
"""COOKED-DRAW submitted-output parity checkpoint, fail closed on the first mismatch.

Compare real production direct with an independently derived opt-in indirect reference.
Selected meshlet-indirect images and unthresholded residuals are retained for inspection.
Run under the repository GPU lock; output must be new.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png(path, rgba, width=1920, height=1080):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    raw = b''.join(b'\0' + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))


VIEWS = [('noon-0', 'valley', 0, 12), ('noon-18', 'valley', 18, 12),
         ('noon-36', 'valley', 36, 12), ('low-sun', 'valley', 0, 6.5),
         ('diagnostic', 'diagnostic', 0, 12)]


def difference(a, b, output):
    if len(a) != len(b):
        raise RuntimeError('different output extents')
    mask = bytearray(b'\0\0\0\xff' * (1920 * 1080))
    changed = 0
    largest = 0
    for offset in range(0, len(a), 4):
        delta = [abs(x-y) for x,y in zip(a[offset:offset+4], b[offset:offset+4])]
        if any(delta):
            changed += 1
            largest = max(largest, *delta)
        mask[offset:offset+4] = bytes(delta)
        mask[offset+3] = 255
    png(output, mask)
    return {'changed_pixels': changed, 'maximum_channel_delta': largest}


def native_crops(a, b, output):
    regions = [(0, 400), (1600, 400), (800, 280), (600, 550)]
    residuals = [(max(abs(x-y) for x,y in zip(a[i:i+4],b[i:i+4])),i//4%1920,i//4//1920)
                 for i in range(0,len(a),4) if a[i:i+4] != b[i:i+4]]
    if residuals:
        _, x, y = max(residuals)
        regions.append((max(0,min(1600,x-160)),max(0,min(824,y-128))))
    rows = []
    for raw in [a,b]:
        rows.extend(b''.join(raw[((y+line)*1920+x)*4:((y+line)*1920+x+320)*4]
                             for x,y in regions) for line in range(256))
    png(output,b''.join(rows),320*len(regions),512)
    return {'regions_xy':regions,'width':320,'height':256,'rows':['I','D']}


def capture(binary, store, output, view, arm, env):
    from cooked_draw_fixture import placements
    name, scene, yaw, solar = view
    masked = arm != 'I'
    modes = {'I': 'production', 'D': 'production', 'R': 'reference', 'omit': 'omit', 'all': 'partitioned'}
    run_env = dict(env, THRUM_SCENE=scene, THRUM_ASSET_STORE=str(store), THRUM_STARS='off',
                   COOKED_DRAW_OUTPUT=str(output), COOKED_DRAW_MASK_INDIRECT=str(int(masked)),
                   COOKED_DRAW_MODE=modes[arm], COOKED_DRAW_YAW=str(yaw), COOKED_DRAW_SOLAR=str(solar),
                   COOKED_DRAW_MESHES=json.dumps(placements(scene)), THRUM_COOKED_DRAW_DIAGNOSTIC='1')
    subprocess.run([str(binary)], env=run_env, check=True)
    row = json.loads((output / 'report.json').read_text())
    expected = 'Some(Direct)' if masked else 'Some(Indirect)'
    if row['path'] != expected or row['drawn'] != 1:
        raise RuntimeError(f'wrong path or undrawn set {row}')
    raw = (output / 'output.rgba').read_bytes()
    png(output / 'output.png', raw)
    row.update(binary_sha256=digest(binary), rgba_sha256=digest(output / 'output.rgba'))
    return raw, row


def paths(out, env):
    from cooked_draw_fixture import sources, cook
    subprocess.run(['cargo', 'build', '--locked', '--offline', '-p', 'v1-game', '--features', 'draw-probe', '--bin', 'cooked_draw_probe'], env=env, check=True)
    source, store = out / 'source', out / 'store'
    sources(source, env)
    cook(source, store, env)
    binary = Path('target/debug/cooked_draw_probe').resolve()
    report = {'target': 'mac-metal', 'extent': [1920,1080], 'pass': False, 'views': [],
              'commit': subprocess.check_output(['git','rev-parse','HEAD'], text=True).strip(),
              'diff_sha256': hashlib.sha256(subprocess.check_output(['git','diff','HEAD'])).hexdigest(),
              'source_hashes': {str(p):digest(p) for root in [Path('crates'),Path('examples')] for p in root.rglob('*') if p.suffix in ['.rs','.wgsl','.toml'] and 'tests' not in p.parts and 'benches' not in p.parts and p.name != 'tests.rs'},
              'content_hashes': {str(p.relative_to(store)):digest(p) for p in store.rglob('*') if p.is_file()}}
    try:
        for view in VIEWS:
            folder = out / view[0]
            arms, data = {}, {}
            for arm in ['I','D','R','all','I-repeat','D-repeat','R-repeat','omit','R-after','D-restored']:
                selected = arm.split('-')[0]
                raw, row = capture(binary, store, folder / arm, view, selected, env)
                arms[arm], data[arm] = row, raw
            exact = data['D'] == data['R']
            repeats = all(data[arm] == data[arm+'-repeat'] for arm in ['I','D','R'])
            negative = data['omit'] != data['R']
            restored = data['D-restored'] == data['R']
            reference_unchanged = data['R-after'] == data['R'] and arms['R-after']['reference_commands'] == arms['R']['reference_commands']
            result = {'view': view, 'arms': arms, 'direct_reference_exact':exact, 'same_arm_repeats_exact':repeats,
                      'missing_mesh_detected':negative, 'restored_exact':restored, 'reference_unchanged_after_defect':reference_unchanged,
                      'indirect_direct':difference(data['I'],data['D'],folder/'I-D-difference.png'),
                      'indirect_all':difference(data['I'],data['all'],folder/'I-all-difference.png'),
                      'all_direct':difference(data['all'],data['D'],folder/'all-D-difference.png'),
                      'native_crops':native_crops(data['I'],data['D'],folder/'native-crops-I-D.png')}
            report['views'].append(result)
            print(view[0], 'D/R exact', exact, 'repeats', repeats, 'negative', negative, 'restored', restored, flush=True)
            if not (exact and repeats and negative and restored and reference_unchanged):
                raise RuntimeError(f'exact oracle/control failed for {view[0]}')
        report['pass'] = True
    finally:
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')


def main():
    from cooked_draw_fixture import environment
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['paths','indirect'])
    parser.add_argument('--target', required=True, choices=['mac-metal','chrome-webgpu'])
    parser.add_argument('--base')
    parser.add_argument('--output', required=True,type=Path)
    args=parser.parse_args()
    os.chdir(ROOT)
    out=args.output.resolve()
    out.mkdir(parents=True,exist_ok=False)
    if args.mode == 'paths' and args.target == 'mac-metal':
        paths(out,environment())
    elif args.mode == 'indirect' and args.target == 'mac-metal' and args.base:
        from cooked_draw_indirect import native
        native(out,args.base,environment())
    elif args.mode == 'indirect' and args.target == 'chrome-webgpu' and args.base:
        from cooked_draw_indirect import browser
        browser(out,args.base,environment())
    else:
        raise RuntimeError('indirect base qualification is not implemented yet')


if __name__ == '__main__':
    main()
