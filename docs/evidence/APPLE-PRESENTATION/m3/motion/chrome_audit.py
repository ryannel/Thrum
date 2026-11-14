"""Original Chrome source verification, ordered inspection crops and diagnostics."""
import argparse
import bisect
import gzip
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[5]/'scripts'))
from t2_writer_probe import package

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source',type=Path,default=Path('tmp/apple-presentation-m3/chrome-motion-final'))
parser.add_argument('--output',type=Path,default=Path('tmp/apple-presentation-m3/chrome-review'))
args = parser.parse_args()
root = args.source
out = args.output
out.mkdir(exist_ok=False)
report = {'scope':'Original gzip RGBA frames, SHA-256 checked. Native-pixel crops are row-major. Flux and lag fits are diagnostics without acceptance thresholds.', 'arms':{}}
linear = [v/255/12.92 if v/255 <= .04045 else ((v/255+.055)/1.055)**2.4 for v in range(256)]
star_boxes = [(1136,112,48,48),(416,0,48,48),(672,160,48,48),(144,56,48,48)]
for case in ['pan','reveal','spawn','stars','scrub']:
    for mode in ['taau','off']:
        name = case+'-'+mode
        session = root/name
        manifest = json.loads((session/'manifest.json').read_text())
        runtime = json.loads((session/'runtime.json').read_text())
        records = manifest['records']
        width,height = runtime['extent']
        assert [width,height] == [1280,720]
        assert manifest['complete'] and runtime['complete']
        times = [(r['submit_ms']-records[0]['submit_ms'])/1000 for r in records]
        sampled = sorted({0,len(records)-1,*[min(range(len(times)),key=lambda i:abs(times[i]-t/2)) for t in range(1,9)]})
        spawn = next((i for i,r in enumerate(records) if r['source_tick'] >= 60),None)
        if case == 'spawn':
            selected = list(range(max(0,spawn-2),min(len(records),spawn+6)))
        else:
            start = min(range(len(times)),key=lambda i:abs(times[i]-(1.8 if case=='pan' else 2.4)))
            selected = list(range(start,min(len(records),start+12)))
        box = (0,360,850,280) if case=='pan' else (350,400,550,240)
        crop_w,crop_h = box[2:]
        columns = 2
        atlas = bytearray(columns*crop_w*math.ceil(len(selected)/columns)*crop_h*4)
        star_atlases = [bytearray(24*48*math.ceil(len(records)/24)*48*4) for _ in star_boxes] if case=='stars' else []
        centres,fluxes = [],[[] for _ in star_boxes]
        whole,trace = [],[]
        def luminance(raw,x,y):
            at = (y*width+x)*4
            return sum(c*linear[raw[at+i]] for i,c in enumerate([.2126,.7152,.0722]))
        for index,row in enumerate(records):
            raw = gzip.decompress((session/row['file']).read_bytes())
            assert len(raw)==width*height*4 and hashlib.sha256(raw).hexdigest()==row['rgba_sha256']
            if index in sampled:
                whole.append(raw)
            if index in selected:
                cell = selected.index(index)
                x,y,w,h = box
                for dy in range(h):
                    target = ((cell//columns*h+dy)*columns*w+cell%columns*w)*4
                    start = ((y+dy)*width+x)*4
                    atlas[target:target+w*4] = raw[start:start+w*4]
            if case=='scrub':
                rgb = [sum(raw[(y*width+x)*4+c] for y in range(64,256,16) for x in range(64,512,16))/(12*28) for c in range(3)]
                trace.append({'time_s':times[index],'sky_rgb':rgb})
            if case=='stars':
                for b,(x,y,w,h) in enumerate(star_boxes):
                    for dy in range(h):
                        target = ((index//24*h+dy)*24*w+index%24*w)*4
                        start = ((y+dy)*width+x)*4
                        star_atlases[b][target:target+w*4] = raw[start:start+w*4]
                    if index==0:
                        centres.append(max(((a,b) for b in range(y+4,y+h-4) for a in range(x+4,x+w-4)),key=lambda p:luminance(raw,*p)))
                    cx,cy = centres[b]
                    background = statistics.median(luminance(raw,a,b) for a,b in [(x+1,y+1),(x+w-2,y+1),(x+1,y+h-2),(x+w-2,y+h-2)])
                    fluxes[b].append(sum(luminance(raw,a,b) for b in range(cy-3,cy+4) for a in range(cx-3,cx+4))-49*background)
        command = ['ffmpeg','-hide_banner','-loglevel','error','-f','rawvideo','-pixel_format','rgba','-video_size',f'{width}x{height}','-i','pipe:0','-vf',f'scale=426:240,tile=3x{math.ceil(len(whole)/3)}','-frames:v','1','-y',str(out/(name+'-whole.png'))]
        subprocess.run(command,input=b''.join(whole),check=True)
        package(atlas,out/(name+'-consecutive.png'),columns*crop_w,math.ceil(len(selected)/columns)*crop_h)
        info = {'frames_verified':len(records),'elapsed_seconds':times[-1],'whole_samples':[{'ordinal':i,'time_s':times[i],'submission_id':records[i]['submission_id']} for i in sampled],
                'native_box':box,'consecutive_samples':[{k:records[i][k] for k in ['ordinal','submit_ms','submission_id','source_tick','alpha','rgba_sha256']} for i in selected],
                'reconstruction':runtime['initial_report']['reconstruction'],'renderer_omissions':runtime['renderer_omissions']}
        if case=='scrub':info['trace']=trace
        if case=='stars':
            info['stars']=[]
            for b,atlas in enumerate(star_atlases):
                package(atlas,out/(name+f'-star-{b}.png'),24*48,math.ceil(len(records)/24)*48)
                values = fluxes[b];mean=statistics.mean(values)
                info['stars'].append({'box':star_boxes[b],'centre':centres[b],'flux_min':min(values),'flux_max':max(values),'flux_mean':mean,'relative_std':statistics.pstdev(values)/abs(mean),'trace':values})
        report['arms'][name]=info
        print(name,len(records),flush=True)
off = report['arms']['scrub-off']['trace']
times = [r['time_s'] for r in off]
def interpolate(t):
    i=max(1,min(len(times)-1,bisect.bisect_left(times,t)))
    a=max(0,min(1,(t-times[i-1])/(times[i]-times[i-1])))
    return [off[i-1]['sky_rgb'][c]*(1-a)+off[i]['sky_rgb'][c]*a for c in range(3)]
fits=[]
for shift in range(-100,101):
    errors=[]
    for r in report['arms']['scrub-taau']['trace']:
        if 1.05 < r['time_s'] < 3.1:
            ref=interpolate(r['time_s']-shift/1000)
            errors.extend((r['sky_rgb'][c]-ref[c])**2 for c in range(3))
    fits.append({'taau_lag_ms':shift,'rgb_rmse_codes':math.sqrt(sum(errors)/len(errors))})
report['scrub_fit']={'best':min(fits,key=lambda x:x['rgb_rmse_codes']),'zero':fits[100],'scope':'Separate elapsed-time runs, submitted-time fit, scheduling and jitter limits; not perceptual acceptance.'}
(out/'run.json').write_text(json.dumps(report,indent=2)+'\n')
print(report['scrub_fit'])
