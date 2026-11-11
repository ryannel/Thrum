from pathlib import Path
import shutil,json,hashlib,subprocess,gzip
r=Path('tmp/g3t-m3-mf1');out=Path('docs/evidence/render/apple-tile-m3/window-rerun');out.mkdir(exist_ok=True)
def cp(src,dst):
 dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst)
for n in ['preflight.json','provenance.json','session.jsonl','inspect-pending.md','source-audit.json','cache-cleanup.json']:
 cp(r/n,out/n)
for recipe in ['portable','apple-tile']:
 for src in (r/'window'/recipe).iterdir():cp(src,out/recipe/src.name)
 for name in ['sun','night','noon']:
  for n in ['checkpoint-0.png','manifest.json','runtime.json']:
   cp(r/'window-references'/f'{recipe}-{name}'/n,out/'references'/f'{recipe}-{name}'/n)
 for src in (r/'interactive').glob(f'{recipe}-*'):
  if src.suffix in ['.png','.json','.log']:cp(src,out/'interactive'/src.name)
 source=r/'window'/f'{recipe}-recovery-settled-video';target=out/f'{recipe}-recovery'
 for n in ['observer.json','game.log','recorder.log','window.txt','window-response.mov','stale-recycle.txt','timeline.json']:
  cp(source/n,target/n)
 for n in ['clock-anchor.tsv','surface.tsv','controls.tsv','retirements.tsv']:
  with gzip.open(target/(n+'.gz'),'wb') as f:f.write((source/n).read_bytes())
 subprocess.run(['ffmpeg','-y','-loglevel','error','-ss','3.7','-i',str(source/'window-response.mov'),'-frames:v','1',str(target/'post-pan.png')],check=True)
cp(Path('tmp/g3t-m2/attended/windows.swift'),out/'reproduction/windows.swift')
for n in ['window.py','window-references.py','recovery-video.py','interactive-config.py','interactive-capture.py','interactive.py','interactive-both.py','run.py','geometry.py','media.py','package.py']:
 cp(r/n,out/'reproduction'/n)
observer={'tool':'/usr/sbin/screencapture','version':'OS bundled; no standalone version flag','sha256':hashlib.sha256(Path('/usr/sbin/screencapture').read_bytes()).hexdigest(),'os':subprocess.check_output(['sw_vers'],text=True),'enumeration_helper_sha256':hashlib.sha256(Path('tmp/g3t-m2/attended/windows').read_bytes()).hexdigest(),'permission_changed':False}
(out/'observer-tool.json').write_text(json.dumps(observer,indent=2)+'\n')
hashes={str(p.relative_to(out)):hashlib.sha256(p.read_bytes()).hexdigest() for p in out.rglob('*') if p.is_file() and p.name!='sha256.json'}
(out/'sha256.json').write_text(json.dumps(hashes,indent=2)+'\n')
print(out,len(hashes))
