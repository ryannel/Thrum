from pathlib import Path
import subprocess,json
r=Path('tmp/g3t-m3-mf1');out=Path('docs/evidence/render/apple-tile-m3/window-rerun');facts={};actions={}
for recipe in ['portable','apple-tile']:
 rows=[]
 for state in ['before','key-pan','resized','restored']:
  p=r/'interactive'/f'{recipe}-{state}.png';info=json.loads(p.with_suffix('.json').read_text());w,h=info['image_extent']
  raw=subprocess.check_output(['ffmpeg','-v','error','-threads','1','-i',str(p),'-f','rawvideo','-pix_fmt','rgb24','-'])
  black=[y-64 for y in range(64,h) if max(raw[(y*w+w//2)*3:(y*w+w//2)*3+3])==0]
  rows.append({'state':state,'image_extent':[w,h],'content_extent':[w,h-64],'center_black_content_rows':black,'pid':info['pid'],'unix_ns':info['unix_ns']})
 assert len(set(row['pid'] for row in rows))==1
 assert [row['image_extent'] for row in rows]==[[1920,1144],[1920,1144],[1520,944],[1920,1144]]
 assert rows[2]['center_black_content_rows']==list(range(12))+list(range(867,880))
 facts[recipe]={'observations':rows,'backing_scale':2,'scene_extent':[1920,1080],'content_bounds_convention':'[left,top,right,bottom] in OS image pixels; 64-pixel title bar excluded'}
 actions[recipe]={'tool':'CUA native app keyboard/drag','actions':['E (5-degree yaw)','Q (reverse yaw)','bottom-right drag [1916,1138] to [1516,938]','reverse drag to [1916,1138]'],'record_start_signal_unix_ns':(r/f'{recipe}-record-start').stat().st_mtime_ns,'signal_scope':'Observer start signal; recorder begins after at most one 250 ms poll plus process/tool startup, not a latency measurement','state_observation_unix_ns':{row['state']:row['unix_ns'] for row in rows},'observed_result':'Visible pan and reversal; centered letterbox, intact geometry and restored original view under the same PID'}
(out/'geometry.json').write_text(json.dumps(facts,indent=2)+'\n');(out/'cua-actions.json').write_text(json.dumps(actions,indent=2)+'\n')
print({k:[(v['state'],v['content_extent']) for v in row['observations']] for k,row in facts.items()})
