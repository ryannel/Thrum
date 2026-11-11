import sys,json,gzip,hashlib,subprocess,shutil
from pathlib import Path
sys.path.insert(0,'scripts')
from capture_spool import decoded_frames
from t2_writer_probe import package
src=Path(sys.argv[1]); out=Path('docs/evidence/ATMOSPHERE/M5-review'); name=src.name
m=json.loads((src/'manifest.json').read_text()); r=json.loads((src/'runtime.json').read_text())
metal=name.startswith('metal'); w,h=r.get('extent',[1280,720]) if metal else r['effective_route']['render_extent']
if metal:
 records=r['images']; times=[v['submit_ns']/1e9 for v in records]
 frames=decoded_frames(src/m['spool_file'],r['spool_encoding'],r['writer_result']['Ok']['frame_sha256'],w*h*4)
else:
 records=m['records']; times=[(i+1)/60 if m['mode']=='fixed' else v['submit_ms']/1000 for i,v in enumerate(records)]
 def read():
  for v in records:
   raw=gzip.decompress((src/v['file']).read_bytes());assert hashlib.sha256(raw).hexdigest()==v['rgba_sha256'];yield raw
 frames=read()
assert r['complete'] if metal else m['complete']
want=[i/10 for i in range(60)]; indices=[min(range(len(times)),key=lambda i:abs(times[i]-t)) for t in want]
samples=[1.25,3.25] if 'west' not in name else [2.,4.]
selected={min(range(len(times)),key=lambda i:abs(times[i]-t)):t for t in samples}
video=subprocess.Popen(['ffmpeg','-v','error','-y','-f','rawvideo','-pixel_format','rgba','-video_size',f'{w}x{h}','-framerate','10','-i','-','-vf','scale=960:-2','-c:v','libx264','-crf','18','-pix_fmt','yuv420p',str(out/f'{name}-10fps.mp4')],stdin=subprocess.PIPE)
selection=[]
for i,raw in enumerate(frames):
 for j in range(indices.count(i)):video.stdin.write(raw)
 if i in selected:
  t=selected[i];fn=f'{name}-{t:.2f}.png';package(raw,out/fn,w,h)
  x,y,cw,ch=int(w*.5),int(h*.2),int(w*.48),int(h*.65)
  crop=b''.join(raw[((y+row)*w+x)*4:((y+row)*w+x+cw)*4] for row in range(ch))
  package(crop,out/f'{name}-{t:.2f}-crop.png',cw,ch)
  selection.append(dict(requested=t,time=times[i],record=records[i],image=fn))
video.stdin.close();assert video.wait()==0
for start in [0,2,4]:
 subprocess.run(['ffmpeg','-v','error','-y','-ss',str(start),'-i',str(out/f'{name}-10fps.mp4'),'-vf','scale=320:180,tile=5x4','-frames:v','1',str(out/f'{name}-strip-{start}.png')],check=True)
for f in ['manifest.json','runtime.json']:
 (out/f'{name}-{f}.gz').write_bytes(gzip.compress((src/f).read_bytes()))
(out/f'{name}-selection.json').write_text(json.dumps(dict(complete=True,verified_frames=len(records),samples=selection,sequence_sample_times=[times[i] for i in indices],scope='10 Hz sampled engine frames; not full video playback or display cadence'),indent=2)+'\n')
# Only owned, verified reconstructible raw capture data is removed to keep allowance.
if metal:(src/m['spool_file']).unlink()
else:shutil.rmtree(src/'frames')
