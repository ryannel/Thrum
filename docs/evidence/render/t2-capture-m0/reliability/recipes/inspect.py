import json,subprocess,hashlib,statistics as st
from pathlib import Path
root=Path('tmp/t2-capture-m0-r3/qualified');out=Path('tmp/t2-capture-m0-r3/inspection');out.mkdir(exist_ok=True);rows=[]
for name in ['series-00','series-01','series-02','render-stall']:
 d=root/name
 if not (d/'png').exists() or not json.loads((d/'manifest.json').read_text()).get('complete'):continue
 r=json.loads((d/'runtime.json').read_text());m=json.loads((d/'manifest.json').read_text());ims=r['images'];rec=m['records'];hashes=[x['rgba_sha256'] for x in rec];times=[x['submit_ns']/1e9 for x in ims]
 hold0=next((i for i,x in enumerate(hashes) if x!=hashes[0]),len(hashes));hold1=next((i for i,x in enumerate(reversed(hashes)) if x!=hashes[-1]),len(hashes));mid=min(range(len(ims)),key=lambda i:abs(times[i]-2.0)); selected=[mid,mid+1]
 row={'session':name,'initial_hold_s':(times[hold0] if hold0<len(times) else r['interval_end_ns']/1e9)-times[0],'final_hold_s':r['interval_end_ns']/1e9-times[-hold1],'max_image_gap_ms':max(b-a for a,b in zip(times,times[1:]))*1000,'selected':[],'history_epochs':sorted(set(x['history_epoch'] for x in ims)),'tick_range':[min(x['tick'] for x in ims),max(x['tick'] for x in ims)],'strict_alpha_count':sum(0<x['alpha']<1 for x in ims),'interior_alpha_one':sum(x['alpha']==1 and 0<x['tick']<119 for x in ims),'actions':r['diagnostic_actions']}
 for i in selected:
  identity=ims[i]['submission_id'];source=d/'png'/f'{identity:06}.png';target=out/f'{name}-{identity}.png';xy='480:360:280:500' if name!='series-02' else '480:360:600:300'
  subprocess.run(['ffmpeg','-y','-v','error','-i',str(source),'-vf','crop='+xy,'-frames:v','1',str(target)],check=True)
  row['selected'].append({'submission_id':identity,'submit_s':times[i],'full_png':str(source),'full_rgba_sha256':hashes[i],'crop':str(target),'crop_rect':xy})
 rows.append(row)
(out/'inspection.json').write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps(rows,indent=2))
