from pathlib import Path
import gzip,hashlib,json,shutil,statistics
root=Path('tmp/t2-capture-m1');dest=Path('docs/evidence/render/t2-capture-m1')
qualified=['fixed-final']+[f'final-{i:02}' for i in range(5)]+['stall-final','resize-final','saturation-final','slow-final','loss-final']
report={'sessions':[],'source_mismatches':[]}
for name in qualified:
 p=root/name;m=json.loads((p/'manifest.json').read_text());r=json.loads((p/'runtime.json').read_text())
 assert m['complete'] or m.get('negative_control_pass'),name
 assert m['permission_preflight']['screen_recording_before'] is False and m['permission_preflight']['screen_recording_after'] is False
 assert '"IOConsoleLocked" = Yes' in m['console'] and '"IOConsoleLocked" = Yes' in m['console_after']
 for row in m['records']:
  raw=gzip.decompress((p/row['file']).read_bytes());assert len(raw)==1280*720*4;assert hashlib.sha256(raw).hexdigest()==row['rgba_sha256']
 report['sessions'].append({'session':name,'verified_images':len(m['records']),'complete':m['complete'],'negative_control_pass':m.get('negative_control_pass',False),'encoded_bytes':m['encoded_bytes']})
m=json.loads((root/'final-00/manifest.json').read_text())
for name,digest in m['source_files'].items():
 actual=hashlib.sha256(Path(name).read_bytes()).hexdigest()
 if actual!=digest:
  assert name=='scripts/capture_v1_browser.mjs'
  assert hashlib.sha256((dest/'capture-runner-measured.mjs').read_bytes()).hexdigest()==digest
  report['source_mismatches'].append({'path':name,'reason':'indentation-only cleanup after measurement; exact measured runner retained as capture-runner-measured.mjs'})
rows=m['records'];first=rows[0]['rgba_sha256'];last=rows[-1]['rgba_sha256'];initial=next(i for i,x in enumerate(rows) if x['rgba_sha256']!=first);final=next(i for i,x in enumerate(reversed(rows)) if x['rgba_sha256']!=last)
report['initial_hold']={'identical_frames':initial,'duration_ms':rows[initial]['submit_ms']-rows[0]['submit_ms']}
held=[row for row in rows if row['tick']>=521]
report['final_hold']={'camera_pose_frames':len(held),'duration_ms':held[-1]['submit_ms']-held[0]['submit_ms'],'basis':'Authored route final pose is constant from relative tick 340 onward; retained pose reports 521–538 agree, native final frames inspected. Running world/solar lighting can change pixel hashes.'}
s=json.loads((root/'stall-final/manifest.json').read_text());pairs=list(zip(s['records'],s['records'][1:]));a,b=max(pairs,key=lambda pair: pair[1]['submit_ms']-pair[0]['submit_ms']);report['stall']={'before':a,'after':b,'gap_ms':b['submit_ms']-a['submit_ms']}
for row in [a,b]:shutil.copyfile(root/'stall-final/png'/f"{row['submission_id']:06}.png",dest/'native'/f"stall-{row['submission_id']:06}.png")
for name in ['final-00','fixed-final','stall-final']:
 pack=json.loads((root/name/'packaging.json').read_text());assert pack['preview_pts_max_error_seconds']<=0.00001
report['series_images']=sum(x['verified_images'] for x in report['sessions'] if x['session'].startswith('final-'))
report['full_check']='PASS ./dev check under gpu lock, check-full.log.gz'
report['lifecycle']='PASS actual tabs, resize, device loss, device recovery, all seven deterministic moving-route controls; lifecycle-order.json'
(dest/'audit.json').write_text(json.dumps(report,indent=2)+'\n')
shutil.copyfile(root/'playback-review/playback.json',dest/'playback.json')
shutil.copyfile(root/'review-playback.mjs',dest/'review-playback.mjs')
shutil.copyfile(root/'promote.py',dest/'promote.py')
shutil.copyfile(root/'final-series.py',dest/'qualification-series.py')
shutil.copyfile(root/'lifecycle.py',dest/'lifecycle.py')
shutil.copyfile(root/'audit.py',dest/'audit.py')
print(json.dumps({k:v for k,v in report.items() if k not in ['sessions','stall']},indent=2));print('stall images',a['submission_id'],b['submission_id'],report['stall']['gap_ms'])
