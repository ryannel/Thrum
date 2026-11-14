from PIL import Image,ImageDraw
import pathlib,json
root=pathlib.Path('tmp/apple-presentation-m3/mac-motion-final');out=pathlib.Path('tmp/apple-presentation-m3/native-review')
for case in ['pan','reveal','spawn','stars','scrub']:
 for mode in ['taau','off']:
  p=root/f'{case}-{mode}'/'session';d=json.loads((p/'runtime.json').read_text());ims={i['submission_id']:i for i in d['images']};files=sorted((p/'png').glob('*.png'))
  if case in ['spawn','reveal','pan']:
   fs=[f for f in files if 1.97<ims[int(f.stem)]['submit_ns']/1e9<2.08]
   if not fs:fs=files[:6]
   box=(500,650,1250,900) if case!='pan' else (100,650,950,900)
  else:
   fs=files[:6];box=(200,50,700,300) if case=='stars' else (0,250,700,600)
  sheet=Image.new('RGB',(box[2]-box[0],(box[3]-box[1]+24)*len(fs)));draw=ImageDraw.Draw(sheet)
  for j,f in enumerate(fs):
   y=j*(box[3]-box[1]+24);sheet.paste(Image.open(f).crop(box),(0,y+24));im=ims[int(f.stem)];draw.text((3,y+3),f'{case}-{mode} {f.name} t={im["submit_ns"]/1e9:.6f} tick={im["tick"]} alpha={im["alpha"]:.3f}',fill='white')
  sheet.save(out/f'{case}-{mode}-crops.png')
