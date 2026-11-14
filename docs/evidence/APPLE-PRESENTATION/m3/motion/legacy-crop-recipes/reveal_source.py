import sys,pathlib,json,subprocess
sys.path.insert(0,'scripts')
from capture_spool import decoded_frames
root=pathlib.Path('tmp/apple-presentation-m3/mac-motion-final');out=pathlib.Path('tmp/apple-presentation-m3/native-review')
for mode in ['taau','off']:
 p=root/f'reveal-{mode}'/'session';m=json.loads((p/'manifest.json').read_text());r=json.loads((p/'runtime.json').read_text());im={i['submission_id']:i for i in r['images']};records=m['records'];wanted=[i for i,x in enumerate(records) if 2.4<=x['submit_ns']/1e9<2.51][:12];frames=[];meta=[]
 gen=decoded_frames(p/'frames.zst','ZstdDelta',[x['rgba_sha256'] for x in records],1920*1080*4)
 for j,raw in enumerate(gen):
  if j in wanted:
   frames.append(b''.join(raw[(y*1920+800)*4:(y*1920+1300)*4] for y in range(650,900)));meta.append(im[records[j]['submission_id']])
  if j>=max(wanted):break
 gen.close()
 cmd=['ffmpeg','-hide_banner','-loglevel','error','-f','rawvideo','-pixel_format','rgba','-video_size','500x250','-i','pipe:0','-vf','tile=3x4','-frames:v','1','-y',str(out/f'reveal-{mode}-consecutive-source.png')]
 subprocess.run(cmd,input=b''.join(frames),check=True)
 (out/f'reveal-{mode}-consecutive-source.json').write_text(json.dumps({'box':[800,650,1300,900],'order':'row-major','frames':meta},indent=2))
 print(mode,len(meta),[(x['submission_id'],x['tick'],round(x['submit_ns']/1e9,6)) for x in meta])
