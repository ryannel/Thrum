"""Every verified held-star source frame, native-pixel atlases and diagnostic flux traces."""
import hashlib
import json
import math
from pathlib import Path
import statistics
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'scripts'))
from capture_spool import decoded_frames
from t2_writer_probe import package

root=Path('tmp/apple-presentation-m3/mac-motion-final')
out=Path('tmp/apple-presentation-m3/star-review');out.mkdir(exist_ok=False)
boxes=[(1712,176,64,64),(640,32,64,64),(1024,256,64,64),(224,96,64,64)]
linear=[(x/255/12.92 if x/255<=.04045 else ((x/255+.055)/1.055)**2.4) for x in range(256)]
report={'scope':'All original lossless held-star frames; row-major native 64x64 crops, 32 atlas columns; trailing empty cells are padding. Flux traces are diagnostic, not acceptance thresholds.','arms':{}}
for mode in ['taau','off']:
    session=root/('stars-'+mode)/'session'
    manifest=json.loads((session/'manifest.json').read_text())
    runtime=json.loads((session/'runtime.json').read_text())
    expected=runtime['writer_result']['Ok']['frame_sha256']
    count=len(expected);rows=math.ceil(count/32);width=height=64
    atlases=[bytearray(32*64*rows*64*4) for _ in boxes]
    centres=[];traces=[[] for _ in boxes]
    def luminance(raw,x,y):
        at=(y*1920+x)*4
        return sum(coefficient*linear[raw[at+c]] for c,coefficient in enumerate([.2126,.7152,.0722]))
    source=session/manifest['spool_file']
    assert hashlib.sha256(source.read_bytes()).hexdigest()==manifest['lossless_source_archive']['sha256']
    for frame,raw in enumerate(decoded_frames(source,runtime['spool_encoding'],expected,1920*1080*4)):
        for index,(x,y,w,h) in enumerate(boxes):
            atlas=atlases[index]
            for dy in range(h):
                begin=((frame//32*h+dy)*(32*w)+frame%32*w)*4
                start=((y+dy)*1920+x)*4
                atlas[begin:begin+w*4]=raw[start:start+w*4]
            if frame==0:
                centres.append(max(((a,b) for b in range(y+4,y+h-4) for a in range(x+4,x+w-4)),key=lambda p:luminance(raw,*p)))
            cx,cy=centres[index]
            background=statistics.median(luminance(raw,a,b) for a,b in [(x+1,y+1),(x+w-2,y+1),(x+1,y+h-2),(x+w-2,y+h-2)])
            flux=sum(luminance(raw,a,b) for b in range(cy-3,cy+4) for a in range(cx-3,cx+4))-49*background
            traces[index].append(flux)
    stats=[]
    for index,atlas in enumerate(atlases):
        png=out/f'{mode}-{index}.png'
        package(atlas,png,32*64,rows*64)
        values=traces[index];mean=statistics.mean(values)
        stats.append({'box':boxes[index],'star_centre':centres[index],'frames':count,'flux_min':min(values),
                      'flux_max':max(values),'flux_mean':mean,'flux_std_fraction':statistics.pstdev(values)/mean,
                      'atlas':png.name,'flux':values})
    report['arms'][mode]={'frames_verified':count,'whole_source_unique_hashes':len(set(expected)),'regions':stats,
                          'submissions':[row['submission_id'] for row in manifest['records']]}
(out/'run.json').write_text(json.dumps(report,indent=2)+'\n')
