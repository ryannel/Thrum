from pathlib import Path
import subprocess,os,json,hashlib
p=Path('crates/thrum-asset-gpu/src/cull.wgsl');original=p.read_text()
env=dict(os.environ,PATH='/Users/ryannel/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:'+os.environ['PATH'],RUSTC_WRAPPER='')
report={'diagnostic_only':'one full-range indirect command per placement; source restored after run'}
try:
 s=original.replace('    draws[base] = range.y;', '    let last = ranges[cull.counts.x - 1u];\n    draws[base] = last.x + last.y;').replace('select(0u, 1u, keep)', 'select(0u, 1u, index == 0u)').replace('draws[base + 2u] = range.x;', 'draws[base + 2u] = 0u;')
 p.write_text(s)
 subprocess.run(['cargo','build','--locked','--offline','-p','v1-game','--bin','cooked_draw_probe'],env=env,check=True)
 folder=Path('tmp/cooked-draw/parity-single-indirect')
 env.update(THRUM_SCENE='valley',THRUM_ASSET_STORE=str(Path('tmp/cooked-draw/paths-failure/store').resolve()),THRUM_STARS='off',COOKED_DRAW_OUTPUT=str(folder))
 subprocess.run(['target/debug/cooked_draw_probe'],env=env,check=True)
 raw=(folder/'output.rgba').read_bytes();report['single_indirect_sha256']=hashlib.sha256(raw).hexdigest()
 for arm in ['indirect','direct']:
  a=(Path('tmp/cooked-draw/paths-failure')/arm/'output.rgba').read_bytes();report[arm+'_different_pixels']=sum(raw[i:i+4]!=a[i:i+4] for i in range(0,len(raw),4))
 print(json.dumps(report,indent=2))
finally:
 p.write_text(original)
 Path('tmp/cooked-draw/single-diagnostic.json').write_text(json.dumps(report,indent=2))
