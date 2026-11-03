from pathlib import Path
import numpy as np,json,hashlib,subprocess,sys
from PIL import Image,ImageDraw
R=Path(__file__).resolve().parent;sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();N=384
old=np.memmap(R.parent/'negative-f30d/instrument/assets/cb-anatomy-384.density.r16f',dtype='<f2',mode='r',shape=(N,N,N));new=np.memmap(R/'cb-anatomy-384.density.r16f',dtype='<f2',mode='r',shape=(N,N,N));d=json.loads((R/'bulk-diagnostics.json').read_text());y=np.where((np.arange(N)+.5)*14000/N>=9900)[0]
differences=sum(int(np.count_nonzero(old[:,i,:]!=new[:,i,:])) for i in y);assert differences==0;d['cap_preservation']={'native_world_height_min_m':9900,'different_half_voxels':differences,'scope':'Native upper cap/downwind fringes unchanged byte-for-byte above this plane; thicker root below may change'};(R/'bulk-diagnostics.json').write_text(json.dumps(d,indent=2)+'\n')
cuts=[c for c in d['linecuts'] if c['cross_axis_offset_m']==0];canvas=Image.new('RGB',(900,6*145+35),(20,25,32));draw=ImageDraw.Draw(canvas);draw.text((8,8),'Native centre-ray q; gray=f30d, cyan=bulk-v2. q-axis0..0.25, horizontal-6..6km. kappa unchanged .0045/m.',fill='white')
for i,c in enumerate(cuts):
 col=0 if c['ray_axis']=='front_Z' else 1;row=i//2;left=col*450+35;top=35+row*145;draw.rectangle((left,top+30,left+384,top+125),outline=(100,100,100));draw.text((left,top),f"{c['ray_axis']} y={c['height_world_m']:.0f}m tau {c['f30d']['optical_depth']:.2f}->{c['bulk_v2']['optical_depth']:.2f}",fill='white')
 for label,color in [('f30d',(150,150,150)),('bulk_v2',(80,220,240))]:
  q=c[label]['q_samples_native'];draw.line([(left+j,top+125-min(v/.25,1)*95) for j,v in enumerate(q)],fill=color,width=1)
canvas.save(R/'native-linecuts.png')
for side in [384,256]:
 p=R/f'cb-anatomy-{side}.json';m=json.loads(p.read_text());m['bulk_diagnostics']={'file':'bulk-diagnostics.json','sha256':sha(R/'bulk-diagnostics.json')};p.write_text(json.dumps(m,indent=2)+'\n')
subprocess.run([sys.executable,str(R/'finalize.py')],check=True)
for side in [384,256]:
 p=R/f'cb-anatomy-{side}.json';m=json.loads(p.read_text());m['bulk_finish_recipe']={'file':'finish-bulk.py','sha256':sha(Path(__file__))}
 if side==256:m['native_reference_metadata']['sha256']=sha(R/'cb-anatomy-384.json')
 p.write_text(json.dumps(m,indent=2)+'\n')
print('Bulk diagnostics and source quality finalized; upper cap identical, source384SHA',sha(R/'cb-anatomy-384.density.r16f'))
