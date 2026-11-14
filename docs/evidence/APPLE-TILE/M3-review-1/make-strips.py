from pathlib import Path
import subprocess,json
root=Path(__file__).resolve().parent;out=root/'strips';out.mkdir(exist_ok=True)
records=[]
for folder in ['native-m1','native-m2']:
 for p in (root/folder).glob('*/preview-1x.mp4'):
  name=folder+'-'+p.parent.name
  dest=out/(name+'-%02d.png')
  if not (out/(name+'-01.png')).exists():
   subprocess.run(['ffmpeg','-v','error','-i',str(p),'-vf','fps=10,scale=320:180,tile=5x8',str(dest)],check=True)
  records.append({'source':str(p.relative_to(root)),'sheets':sorted(str(s.relative_to(root)) for s in out.glob(name+'-*.png')),'sampling':'10 fps, row-major, 40 frames / sheet, original elapsed-time movie'})
(root/'strip-index.json').write_text(json.dumps(records,indent=2)+'\n')
print(json.dumps(records))
