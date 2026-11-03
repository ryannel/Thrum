from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parent
files={str(p.relative_to(root)):{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(root.rglob('*')) if p.is_file() and '__pycache__' not in p.parts and p.name!='manifest.json'}
record={'schema':1,'scope':'Two rejected frozen storm-anatomy inputs, no lifecycle/art/performance admission','file_count':len(files),'retained_bytes_excluding_manifest':sum(v['bytes'] for v in files.values()),'files':files}
(root/'manifest.json').write_text(json.dumps(record,indent=2)+'\n')
print('Retained',len(files),'files;',record['retained_bytes_excluding_manifest'],'bytes excluding manifest.')
