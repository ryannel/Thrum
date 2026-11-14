from pathlib import Path
import json
root=Path(__file__).resolve().parent
summary={}
for folder in ['native-m1','native-m2']:
 p=root/folder/'manifest.json'
 if p.exists():
  m=json.loads(p.read_text());summary[folder]={'passed':m['passed'],'comparisons':len(m['comparisons']),'failures':m['failures'],'binary_sha256':m['binary_sha256']}
for p in root.glob('native-*/*-motion/runtime.json'):
 x=json.loads(p.read_text());images=x.get('images',[]); gpu=x.get('gpu',[])
 d={'images':len(images),'history_epochs':sorted(set(i['history_epoch'] for i in images)),'stalls':x.get('diagnostic_actions'),'complete':x.get('complete'),'gpu_samples':len(gpu)}
 if images:
  d['elapsed_seconds']=(images[-1]['submit_ns']-images[0]['submit_ns'])/1e9
 if gpu:
  names=[g.get('gpu_pass_ns') for g in gpu];d['named_timing_samples']=sum(isinstance(n,dict) for n in names)
  d['stars_present']=sum(isinstance(n,dict) and 'atmosphere stars' in n for n in names)
  d['untimed_names']=sorted({k for n in names if isinstance(n,dict) for k,v in n.items() if v is None})
 summary[str(p.parent.relative_to(root))]=d
(root/'independent-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
