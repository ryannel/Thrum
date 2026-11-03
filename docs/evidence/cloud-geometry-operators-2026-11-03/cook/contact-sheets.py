#!/usr/bin/env python3
"""Regenerate native-pixel ROI contacts from retained actual canvas videos (no resizing)."""
from pathlib import Path
import argparse,hashlib,json,subprocess
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
if a.output.exists():raise RuntimeError('Refusing existing output directory')
a.output.mkdir(parents=True)
records=[]
fixtures=[(op,label,rate,seconds,'173d261dc10b86c2','uniform') for op,label in [(1,'sdf'),(3,'alpha')] for rate,seconds in [(1,120),(16,12)]]+[(1,'sdf-noisy',16,12,'81c3f43f76b490fa','noisy')]
for operator,label,rate,seconds,build,channel in fixtures:
 files=list((ROOT/'observations/motion').glob(f'geometry-motion-op{operator}-{rate}x-{seconds}s-{build}-*.webm'))
 if len(files)!=1:raise RuntimeError('Ambiguous source clip')
 source=files[0];filter=('fps=1/15' if rate==1 else 'fps=1')+',crop=512:320:384:200,tile='+('4x2' if rate==1 else '4x3')
 target=a.output/f'{label}-{rate}x-native-roi.png'
 command=['ffmpeg','-hide_banner','-loglevel','error','-i',str(source),'-vf',filter,'-frames:v','1',str(target)]
 subprocess.run(command,check=True)
 records.append({'file':target.name,'sha256':hashlib.sha256(target.read_bytes()).hexdigest(),'bytes':target.stat().st_size,'source_webm':str(source.relative_to(ROOT)),'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'build':build,'channels':channel,'inspection_scope':'Native-pixel contact derivative; continuous watching is not claimed by contact generation.','argv':command,'filter':filter,'crop_xywh':[384,200,512,320],'resize':False,'timing_scope':'FFmpeg fps-filter selection/rounding semantics; these tiles are not individually exact source-clock timestamps or independent frames rendered at stipulated times. Read the original clip and clock receipt.'})
(a.output/'contacts.json').write_text(json.dumps({'ffmpeg_version':subprocess.check_output(['ffmpeg','-version'],text=True).splitlines()[:2],'contacts':records},indent=2)+'\n')
print('Generated five native ROI contacts; source/crop/filter/version/hashes recorded')
