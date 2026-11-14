from PIL import Image,ImageDraw
import pathlib,json
root=pathlib.Path('tmp/apple-presentation-m3/mac-motion-final/playback');out=pathlib.Path('tmp/apple-presentation-m3/native-review')
for p in sorted(root.glob('*')):
 if not (p/'playback.json').exists():continue
 d=json.loads((p/'playback.json').read_text());fs=d['samples'];sheet=Image.new('RGB',(1440,294*((len(fs)+2)//3)));draw=ImageDraw.Draw(sheet)
 for j,f in enumerate(fs):
  x=(j%3)*480;y=(j//3)*294;sheet.paste(Image.open(p/f['image']).resize((480,270)),(x,y+24));draw.text((x+3,y+3),f'{p.name} {f["time_s"]:.4f}s',fill='white')
 sheet.save(out/f'{p.name}-playback.png')
 box=(500,650,1250,900) if p.name.startswith(('reveal','spawn')) else ((200,50,700,300) if p.name.startswith('stars') else ((0,250,700,600) if p.name.startswith('scrub') else (0,650,850,900)))
 sheet=Image.new('RGB',((box[2]-box[0])*3,(box[3]-box[1]+24)*((len(fs)+2)//3)));draw=ImageDraw.Draw(sheet)
 for j,f in enumerate(fs):
  x=(j%3)*(box[2]-box[0]);y=(j//3)*(box[3]-box[1]+24);sheet.paste(Image.open(p/f['image']).crop(box),(x,y+24));draw.text((x+3,y+3),f'{p.name} {f["time_s"]:.4f}s',fill='white')
 sheet.save(out/f'{p.name}-playback-native.png')
 print(p.name,d['quality'])
