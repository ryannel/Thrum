"""Retain selected lossless review frames, receipt records and original-time previews."""
from pathlib import Path
import gzip, hashlib, json, shutil, subprocess, sys
root=Path.cwd()
sys.path.insert(0,str(root/'scripts'))
from t2_writer_probe import package
src=root/'tmp/culling-m3/review-21aa3fb7'
dst=root/'docs/evidence/CULLING/M3-review'
receipt={'revision':'21aa3fb764d1931b88ba2d08f919cd290d7adc82','selections':[],'timing':{}}
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def png(source,target):
    subprocess.run(['ffmpeg','-v','error','-y','-i',str(source),'-frames:v','1',str(target)],check=True)
def keep(source,target):
    shutil.copyfile(source,target)
    receipt['selections'].append({'file':str(target.relative_to(dst)),'source':str(source.relative_to(root)),'sha256':sha(target)})
for kind in ['native-fixed-route','chrome-fixed','native-realtime','chrome-realtime']:
    source=src/kind; target=dst/kind;target.mkdir(exist_ok=True)
    for name in ['runtime.json','manifest.json','packaging.json','route.jsonl']:
        p=source/name
        if p.exists(): (target/(name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
    if kind=='native-fixed-route':
        for p in sorted((source/'checkpoints').glob('*.ppm')):
            q=target/(p.stem+'.png');png(p,q)
            receipt['selections'].append({'file':str(q.relative_to(dst)),'source':str(p.relative_to(root)),'source_sha256':sha(p),'sha256':sha(q)})
        for p in (source/'checkpoints').glob('*.identity.json'):keep(p,target/p.name)
        continue
    d=json.loads((source/'runtime.json').read_text());m=json.loads((source/'manifest.json').read_text())
    rows=d['images']
    times=[r.get('submit_ns',r.get('submit_ms',0)*1e6)/1e6 for r in rows]
    gaps=sorted(b-a for a,b in zip(times,times[1:]))
    receipt['timing'][kind]={'frames':len(rows),'extent':d['extent'],'complete':d['complete'],'median_submission_gap_ms':gaps[len(gaps)//2],'p95_submission_gap_ms':gaps[int(len(gaps)*.95)],'max_submission_gap_ms':max(gaps),'scope':'engine submissions with capture ON; not physical display cadence'}
    def frame(row):return source/'png'/f"{row['submission_id']:06}.png"
    if kind=='chrome-fixed':
        for tick in [225,350,376,386,530]:
            row=min(rows,key=lambda r:abs(r['tick']-tick)); keep(frame(row),target/f'checkpoint-{tick}.png')
        continue
    for name in ['preview-1x.mp4','contact.png']:keep(source/name,target/name)
    subprocess.run(['ffmpeg','-v','error','-y','-i',str(source/'preview-1x.mp4'),'-vf','fps=10,scale=320:180,tile=5x8','-fps_mode','vfr',str(target/'motion-%02d.png')],check=True)
    for tick in [285,375,420]:
        index=min(range(len(rows)),key=lambda i:abs(rows[i]['tick']-tick))
        selected=rows[index:index+6]
        keep(frame(selected[0]),target/f'whole-{tick}.png')
        tmp=src/f'select-{kind}-{tick}';tmp.mkdir(exist_ok=True)
        for i,row in enumerate(selected):
            p=tmp/f'{i:02}.png'
            if not p.exists():p.symlink_to(frame(row))
        width,height=d['extent']
        # Three regions of each consecutive source frame, in rows at original pixel scale.
        crop_specs=[(0,round(height*.38)),(width//2-160,round(height*(.32 if tick == 420 else .1))),(width-320,round(height*.38))]
        strip=bytearray(960*240*6*4)
        for i,row in enumerate(selected):
            raw=subprocess.check_output(['ffmpeg','-v','error','-i',str(frame(row)),'-f','rawvideo','-pix_fmt','rgba','-'])
            for column,(x,y) in enumerate(crop_specs):
                for v in range(240):
                    a=((y+v)*width+x)*4;b=((i*240+v)*960+column*320)*4
                    strip[b:b+1280]=raw[a:a+1280]
        q=target/f'crops-{tick}-1x.png';package(bytes(strip),q,960,1440)
        receipt['selections'].append({'file':str(q.relative_to(dst)),'sha256':sha(q),'regions_xywh':[[x,y,320,240] for x,y in crop_specs],'layout':'six consecutive frames, top to bottom; left/centre/right regions left to right; no resampling','frames':[{'submission_id':r['submission_id'],'tick':r['tick'],'source_png_sha256':sha(frame(r))} for r in selected]})
(dst/'selection.json').write_text(json.dumps(receipt,indent=2)+'\n')
