from pathlib import Path
import subprocess,os,json,hashlib
p=Path('crates/thrum-asset-gpu/src/cull.wgsl');original=p.read_text()
env=dict(os.environ,PATH='/Users/ryannel/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:'+os.environ['PATH'],RUSTC_WRAPPER='')
report={'diagnostic_only':'all meshlets retained; source restored after run'}
try:
 p.write_text(original.replace('    let range = ranges[index];','    keep = true; // scratch diagnostic, never integrated\n    let range = ranges[index];'))
 subprocess.run(['cargo','build','--locked','--offline','-p','v1-game','--bin','cooked_draw_probe'],env=env,check=True)
 folder=Path('tmp/cooked-draw/parity-all-meshlets')
 env.update(THRUM_SCENE='valley',THRUM_ASSET_STORE=str(Path('tmp/cooked-draw/paths-failure/store').resolve()),THRUM_STARS='off',COOKED_DRAW_OUTPUT=str(folder))
 subprocess.run(['target/debug/cooked_draw_probe'],env=env,check=True)
 raw=(folder/'output.rgba').read_bytes()
 report['all_meshlets']=json.loads((folder/'report.json').read_text())
 report['all_meshlets_sha256']=hashlib.sha256(raw).hexdigest()
 for arm in ['indirect','direct']:
  other=Path('tmp/cooked-draw/paths-failure')/arm/'output.rgba'
  a=other.read_bytes();report[arm+'_different_pixels']=sum(raw[i:i+4]!=a[i:i+4] for i in range(0,len(raw),4))
 print(json.dumps(report,indent=2))
finally:
 p.write_text(original)
 Path('tmp/cooked-draw/cull-diagnostic.json').write_text(json.dumps(report,indent=2))
