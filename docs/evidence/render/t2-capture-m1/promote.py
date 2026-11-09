import gzip,hashlib,json,shutil,statistics
from pathlib import Path
root=Path('tmp/t2-capture-m1'); dest=Path('docs/evidence/render/t2-capture-m1');dest.mkdir(exist_ok=True)
(dest/'sessions').mkdir(exist_ok=True)
rows=[]
for p in sorted(root.iterdir()):
 if not p.is_dir() or not (p/'manifest.json').exists():continue
 m=json.loads((p/'manifest.json').read_text());r=json.loads((p/'runtime.json').read_text()) if (p/'runtime.json').exists() else {}
 for f in ['manifest.json','runtime.json','chrome.log','launcher.json']:
  if (p/f).exists(): (dest/'sessions'/(p.name+'-'+f+'.gz')).write_bytes(gzip.compress((p/f).read_bytes(),mtime=0))
 if (root/(p.name+'.log')).exists(): (dest/'sessions'/(p.name+'-command.log.gz')).write_bytes(gzip.compress((root/(p.name+'.log')).read_bytes(),mtime=0))
 images=m.get('records',[]);gaps=[b['submit_ms']-a['submit_ms'] for a,b in zip(images,images[1:]) if a and b]
 cpu=[x['returned_ms']-x['requested_ms'] for x in r.get('opportunities',[])]
 rows.append({'session':p.name,'complete':m['complete'],'negative_control_pass':m.get('negative_control_pass',False),'observer_only':m.get('observer_only',False),'error':m.get('error'),'image_count':len(images),'encoded_bytes':m.get('encoded_bytes',0),'max_submit_gap_ms':max(gaps,default=0),'step_wall_median_ms':statistics.median(cpu) if cpu else None,'export_peak':r.get('export_peak'),'receiver_peak':m.get('export_peak'),'renderer_omissions':r.get('renderer_omissions'),'export_omissions':r.get('export_omissions',0),'initial_tick':r.get('initial_report',{}).get('tick'),'final_tick':r.get('final_report',{}).get('tick'),'final_presented':r.get('final_report',{}).get('presented'),'overloads':r.get('final_report',{}).get('overload_events'),'final_sync_ms':m.get('final_sync_ms'),'actions':r.get('actions',[]),'permission':m.get('permission_preflight'),'wasm_sha256':m.get('artifacts',{}).get('/v1_game_bg.wasm'),'capture_driver_sha256':m.get('artifacts',{}).get('/capture.mjs')})
(dest/'summary.json').write_text(json.dumps(rows,indent=2)+'\n')
for f in ['order.json','final-order.json','lifecycle-order.json','inspection-list.md','m0-closeout-dry-run.json','driver-before-saturation-fix.mjs']:
 shutil.copyfile(root/f,dest/f)
for p in root.glob('legacy-*'):
 if p.is_dir():
  for f in p.iterdir():
   if f.suffix in ['.json','.log'] and f.is_file(): (dest/'sessions'/(p.name+'-'+f.name+'.gz')).write_bytes(gzip.compress(f.read_bytes(),mtime=0))
for f in ['check-full.log','writer-tests.log','baseline.log','sandbox-baseline.log','restricted-pilot.log','final-series.log','lifecycle.log']:
 (dest/(f+'.gz')).write_bytes(gzip.compress((root/f).read_bytes(),mtime=0))
print('promoted metadata',len(rows))
