from pathlib import Path
import subprocess,json,sys,shutil,hashlib
root=Path.cwd(); out=root/'docs/evidence/APPLE-TILE/M3-review/current'; scratch=root/'tmp/apple-tile-m3-review-current'
sys.path.insert(0,str(root/'scripts'));from package_v1_browser import run as package
report={}
for scene,control,name in [('day','ordinary','chrome-day'),('night','ordinary','chrome-night'),('day','resize','chrome-resize')]:
 dest=scratch/name
 observer='docs/evidence/APPLE-TILE/M3-review/chrome-resize-observer.mjs' if control=='resize' else 'scripts/capture_v1_browser.mjs'
 command=['node',observer,'--mode','realtime','--scene',scene,'--control',control,'--cook','target/debug/thrum-cook','--output',str(dest)]
 with (out/(name+'.log')).open('w') as f:r=subprocess.run(command,stdout=f,stderr=subprocess.STDOUT)
 if r.returncode:raise RuntimeError(name)
 package(dest)
 m=json.loads((dest/'manifest.json').read_text());assert m['complete'];rows=m['records']
 keep={0,len(rows)//2,len(rows)-1}
 for i in range(1,len(rows)):
  if any(rows[i].get(k)!=rows[i-1].get(k) for k in ['history_epoch','presenter_generation']):keep.update(range(max(0,i-2),min(len(rows),i+3)))
 selected={f"{rows[i]['submission_id']:06}.png" for i in keep}
 for p in (dest/'png').glob('*.png'):
  if p.name not in selected:p.unlink()
 subprocess.run(['ffmpeg','-v','error','-i',str(dest/'preview-1x.mp4'),'-vf','fps=10,scale=384:216,tile=5x8','-fps_mode','vfr',str(dest/'inspection-%02d.png')],check=True)
 retained=out/name;retained.mkdir()
 for p in dest.iterdir():
  if p.is_file() and p.suffix in ['.json','.png','.mp4','.ffconcat','.log']:shutil.copy2(p,retained/p.name)
 shutil.copytree(dest/'png',retained/'png')
 report[name]={'command':command,'complete':m['complete'],'images':len(rows),'selected':sorted(selected),'histories':sorted({r['history_epoch'] for r in rows}),'presenters':sorted({r['presenter_generation'] for r in rows})}
 (out/'chrome-results.json').write_text(json.dumps(report,indent=2)+'\n')
 # Verified original frames and selected PNGs; retained host timeline and hashes precede cleanup.
 shutil.rmtree(dest)
print(json.dumps(report))
