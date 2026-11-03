from pathlib import Path
import argparse,json,subprocess,io
from PIL import Image,ImageDraw
ap=argparse.ArgumentParser();ap.add_argument('clip',type=Path);ap.add_argument('--step',type=int,default=10);ap.add_argument('--duration',type=int,default=90);a=ap.parse_args();frames=[]
for t in range(0,a.duration,a.step):
 b=subprocess.check_output(['ffmpeg','-v','error','-ss',str(t),'-i',str(a.clip),'-frames:v','1','-f','image2pipe','-vcodec','png','-threads','1','-'])
 im=Image.open(io.BytesIO(b)).convert('RGB');im=im.crop((360,160,1240,490));frames.append((t,im))
w,h=880,355;out=Image.new('RGB',(w*2,h*((len(frames)+1)//2)),(18,24,31));d=ImageDraw.Draw(out)
for i,(t,im) in enumerate(frames):
 x=(i%2)*w;y=(i//2)*h;out.paste(im,(x,y+25));d.text((x+8,y+5),'wall '+str(t)+'s · actual decoded native crop',fill=(235,240,245))
target=a.clip.with_suffix('.contact.png');out.save(target);print(target)
info=json.loads(subprocess.check_output(['ffprobe','-v','error','-select_streams','v:0','-count_frames','-show_entries','stream=width,height,nb_read_frames,r_frame_rate:format=duration','-of','json',str(a.clip)]));print(json.dumps(info))
a.clip.with_suffix('.decode.json').write_text(json.dumps(info,indent=2)+'\n')
