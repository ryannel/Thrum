"""Select lossless inspection frames after exact comparison or full source verification."""
import sys,json,gzip,hashlib,shutil
from pathlib import Path
sys.path.insert(0,'scripts')
from t2_writer_probe import package
p=Path(sys.argv[1]);m=json.loads((p/'manifest.json').read_text());r=json.loads((p/'runtime.json').read_text())
assert m['complete'] and r['complete']
rows=m['records'];sel=p/'selected';sel.mkdir(exist_ok=True)
route=json.loads(Path('examples/v1-game/routes/moving-v1.json').read_text())
choices={c['name']:min(rows,key=lambda x:abs(x['source_tick']+x['alpha']-(route['origin_tick']+c['tick']+.5))) for c in route['checkpoints']}
choices['first']=rows[0];choices['last']=rows[-1]
selected=[]
for name,row in choices.items():
 raw=gzip.decompress((p/row['file']).read_bytes());assert hashlib.sha256(raw).hexdigest()==row['rgba_sha256']
 if m['source']=='sdr':package(raw,sel/(name+'.png'),*r['extent'])
 elif name=='mirror-light':shutil.copyfile(p/row['file'],sel/'mirror-light.rgba16f.gz')
 selected.append({'name':name,'record':row})
(sel/'selection.json').write_text(json.dumps(selected,indent=2)+'\n')
# Exact fixed comparison or package_v1_browser has already verified the whole source.
(p/'selection-retention.json').write_text(json.dumps({'selected':list(choices),'verified_records':len(rows),'removed_bulk':'frames and expanded png duplicates','retained':'selected lossless samples, complete identities/hashes and realtime preview where produced'},indent=2)+'\n')
shutil.rmtree(p/'frames')
if (p/'png').exists():shutil.rmtree(p/'png')
