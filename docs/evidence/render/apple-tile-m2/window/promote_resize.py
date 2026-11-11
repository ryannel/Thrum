from pathlib import Path
import json,shutil,subprocess
src=Path('tmp/g3t-m2/attended/visible');dst=Path('docs/evidence/render/apple-tile-m2/window');report={}
for recipe in ['portable','apple-tile']:
 rows=[]
 for name in ['before','resized','restored']:
  p=src/f'{recipe}-{name}.png'
  stream=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-of','json',str(p)]))['streams'][0];w,h=stream['width'],stream['height']
  raw=subprocess.check_output(['ffmpeg','-v','error','-i',str(p),'-f','rawvideo','-pix_fmt','rgb24','-'])
  black=[y for y in range(64,h) if max(raw[(y*w+w//2)*3:(y*w+w//2)*3+3])==0]
  rows.append({'name':name,'image_extent':[w,h],'content_extent':[w,h-64],'center_black_rows':black,'metadata':f'{recipe}/{name}.json'})
  shutil.copy2(p,dst/recipe/f'{name}.png');shutil.copy2(src/f'{recipe}-{name}.json',dst/recipe/f'{name}.json')
 shutil.copy2(src/f'{recipe}-game.log',dst/recipe/'resize-game.log')
 report[recipe]={'observations':rows,'scene_extent':[1920,1080],'backing_scale':2,'resize_UI':'Computer Use drag bottom-right [1916,1138] -> [1516,938], then reverse; app screenshot pixels','inspection':'Centered scene preserved mirror sphere, cube arrangement, rock, shadows and terrain; both recipes restored original full view. At 1520x880 content, a 1520x855 16:9 scene has 12/13 black rows.'}
(dst/'resize.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
