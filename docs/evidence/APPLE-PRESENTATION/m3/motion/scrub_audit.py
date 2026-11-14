"""Inspect every lossless scrub frame; diagnostic lag fit, not an acceptance oracle."""
import argparse
import bisect
import json
import math
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[5]/'scripts'))
from capture_spool import decoded_frames
from t2_writer_probe import package

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source',type=Path,default=Path('tmp/apple-presentation-m3/mac-motion-final'))
parser.add_argument('--output',type=Path,default=Path('tmp/apple-presentation-m3/scrub-review'))
args = parser.parse_args()
root = args.source
out = args.output
out.mkdir(exist_ok=False)
report = {'scope': 'All original lossless Mac scrub frames; original submission times; sky RGB diagnostic, no acceptance threshold.', 'arms': {}}
for mode in ['taau', 'off']:
    session = root / ('scrub-' + mode) / 'session'
    manifest = json.loads((session / 'manifest.json').read_text())
    runtime = json.loads((session / 'runtime.json').read_text())
    records = manifest['records']
    trace = []
    columns, w, h = 32, 64, 32
    rows = math.ceil(len(records) / columns)
    atlas = bytearray(columns*w*rows*h*4)
    for index, raw in enumerate(decoded_frames(session/manifest['spool_file'], runtime['spool_encoding'], [r['rgba_sha256'] for r in records], 1920*1080*4)):
        rgb = [sum(raw[(y*1920+x)*4+c] for y in range(64, 256, 16) for x in range(64, 512, 16))/(12*28) for c in range(3)]
        trace.append({'submit_ns': records[index]['submit_ns'], 'submission_id': records[index]['submission_id'], 'sky_rgb': rgb})
        for y in range(h):
            target = ((index//columns*h+y)*columns*w+index%columns*w)*4
            start = ((96+y)*1920+224)*4
            atlas[target:target+w*4] = raw[start:start+w*4]
    package(atlas, out/(mode+'-sky.png'), columns*w, rows*h)
    report['arms'][mode] = {'frames': len(records), 'trace': trace}
off = report['arms']['off']['trace']
times = [x['submit_ns']/1e9 for x in off]
def interpolate(t):
    i = max(1, min(len(times)-1, bisect.bisect_left(times,t)))
    a = max(0, min(1, (t-times[i-1])/(times[i]-times[i-1])))
    return [off[i-1]['sky_rgb'][c]*(1-a)+off[i]['sky_rgb'][c]*a for c in range(3)]
fits = []
for shift_ms in range(-100,101):
    errors = []
    for row in report['arms']['taau']['trace']:
        t = row['submit_ns']/1e9
        if not 1.05 < t < 3.1:
            continue
        reference = interpolate(t-shift_ms/1000)
        errors.extend((row['sky_rgb'][c]-reference[c])**2 for c in range(3))
    fits.append({'taau_lag_ms': shift_ms, 'rgb_rmse_codes': math.sqrt(sum(errors)/len(errors))})
report['diagnostic_fit'] = {'best': min(fits,key=lambda x:x['rgb_rmse_codes']), 'zero_lag': fits[100], 'meaning': 'Separate elapsed-time arms; control/submit scheduling and jitter affect fit. Positive lag compares TAAU against an earlier off frame. Not a subjective visual pass.'}
(out/'run.json').write_text(json.dumps(report,indent=2)+'\n')
print(report['diagnostic_fit'])
