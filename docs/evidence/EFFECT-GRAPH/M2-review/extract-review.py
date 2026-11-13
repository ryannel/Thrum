import sys,json,hashlib,subprocess,gzip
from pathlib import Path
sys.path.insert(0,'scripts')
from capture_spool import decoded_frames
p=Path('docs/evidence/EFFECT-GRAPH/M2-review'); c=p/'capture'
r=json.loads((c/'runtime.json').read_text()); m=json.loads((c/'manifest.json').read_text()); rows=r['images']; selected={}
for label,tick in [('moving',240),('stop',270),('cadence',300),('wrap',346),('faded',480)]:
 i=min(range(len(rows)),key=lambda i:abs(rows[i]['tick']-tick))
 selected[label]=list(range(max(0,i-3),min(len(rows),i+9))) if label in ['stop','cadence','wrap'] else [i]
w,h=m['extent']; all_selected={i for v in selected.values() for i in v}; identities=[]
for i,raw in enumerate(decoded_frames(c/m['spool_file'],r['spool_encoding'],r['writer_result']['Ok']['frame_sha256'],w*h*4)):
 if i not in all_selected: continue
 out=p/f'frame-{i:04}.png'
 subprocess.run(['ffmpeg','-v','error','-f','rawvideo','-pixel_format','rgba','-video_size',f'{w}x{h}','-i','pipe:0','-frames:v','1','-y',str(out)],input=raw,check=True)
 identities.append(dict(index=i,file=out.name,rgba_sha256=hashlib.sha256(raw).hexdigest(),image=rows[i]))
for label,indices in selected.items():
 if len(indices)<2: continue
 # A broad ground crop at native scale; layout retains time order, left to right then down.
 command=['ffmpeg','-v','error']
 for i in indices: command+=['-i',str(p/f'frame-{i:04}.png')]
 x,y={'stop':(600,90),'cadence':(0,90),'wrap':(1400,100)}[label]
 filters=[f'[{j}:v]crop=512:256:{x}:{y}[c{j}]' for j in range(len(indices))]
 layout='|'.join(f'{(j%3)*512}_{(j//3)*256}' for j in range(len(indices)))
 filters+=[''.join(f'[c{j}]' for j in range(len(indices)))+f'xstack=inputs={len(indices)}:layout={layout}[out]']
 subprocess.run(command+['-filter_complex',';'.join(filters),'-map','[out]','-frames:v','1','-y',str(p/f'{label}-consecutive.png')],check=True)
result={'source_revision':m['source_revision'],'binary_sha256':m['binary_sha256'],'mode':m['timebase'],'adapter':r['adapter'],'backend':r['backend'],'extent':m['extent'],'images_verified':len(rows),'ring_omissions':r['ring_omissions'],'writer_omissions':r['writer_omissions'],'selection':selected,'frames':identities,'source_archive_sha256':hashlib.sha256((c/m['spool_file']).read_bytes()).hexdigest()}
(p/'selected.json').write_text(json.dumps(result,indent=2)+'\n')
for name in ['manifest.json','runtime.json']:
 with gzip.open(p/(name+'.gz'),'wb') as f: f.write((c/name).read_bytes())
print(json.dumps({k:v for k,v in result.items() if k!='frames'},indent=2))
