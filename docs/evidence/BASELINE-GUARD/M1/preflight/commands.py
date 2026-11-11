import gzip, hashlib, json, os, pathlib, subprocess, time
root = pathlib.Path.cwd()
out = root / 'tmp/baseline-guard/preflight-native-browser'
out.mkdir()
def run(name, command):
    start = time.monotonic()
    with (out / (name + '.log')).open('w') as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, env=dict(os.environ, RUSTC_WRAPPER=''))
    print(name, result.returncode, round(time.monotonic()-start, 2), flush=True)
    if result.returncode: raise SystemExit(result.returncode)
run('native-build', ['rustup','run','1.99.1','cargo','build','--locked','--offline','--release','-p','v1-game','--features','draw-probe,thrum-render/creation-counters'])
run('cooker-build', ['rustup','run','1.99.1','cargo','build','--locked','--offline','--release','-p','thrum-cook'])
run('web-build', ['sh','examples/v1-game/web/build.sh'])
run('cook', ['target/release/thrum-cook','cook','examples/v1-game/assets/showcase',str(out/'valley')])
for matrix in ['m1','m2']:
    run(matrix, ['python3',f'scripts/apple_tile_{matrix}.py','--store',str(out/'valley'),'--output',str(out/matrix)])
refs = [
('day','sdr','docs/evidence/render/apple-tile-m0/chrome-baseline/prechange-chrome-fixed-manifest.json.gz'),
('day','scene-linear','docs/evidence/render/apple-tile-m1/chrome/chrome-prechange-linear-3/manifest.json.gz'),
('night','sdr','docs/evidence/render/apple-tile-m1/chrome/chrome-prechange-night-sdr/manifest.json.gz'),
('night','scene-linear','docs/evidence/render/apple-tile-m1/chrome/chrome-prechange-night-linear/manifest.json.gz')]
import sys
sys.path.insert(0, str(root/'scripts'))
from apple_tile_m1_compare_browser import IDENTITY
for scene,source,ref in refs:
    name=f'chrome-{scene}-{source}'
    run(name,['node','scripts/capture_v1_browser.mjs','--mode','fixed','--scene',scene,'--source',source,'--cook','target/release/thrum-cook','--output',str(out/name)])
    before=json.loads(gzip.decompress((root/ref).read_bytes()))
    after=json.loads((out/name/'manifest.json').read_text())
    keys=IDENTITY+(['pixel_source','exposure_gpu_words'] if source=='scene-linear' else [])
    report={'baseline':ref,'owner':'APPLE-TILE','comparison':'retained identity and decoded SHA-256','complete':after['complete'],'records':len(after['records']),'failures':[]}
    if not before['complete'] or not after['complete'] or before['mode']!='fixed' or after['mode']!='fixed': report['failures'].append('incomplete fixed capture')
    if len(before['records'])!=481 or len(after['records'])!=481: report['failures'].append('record count')
    for a,b in zip(before['records'],after['records'],strict=True):
        if any(a.get(k)!=b.get(k) for k in keys): report['failures'].append(f"identity changed at {a['ordinal']}")
        digest=hashlib.sha256(gzip.decompress((out/name/b['file']).read_bytes())).hexdigest()
        if digest!=a['rgba_sha256']: report['failures'].append(f"decoded image changed at {a['ordinal']}: {a['rgba_sha256']} != {digest}")
    (out/(name+'-comparison.json')).write_text(json.dumps(report,indent=2)+'\n')
    print(name,'comparison',len(report['failures']),'failures',flush=True)
    if report['failures']: raise SystemExit(1)
