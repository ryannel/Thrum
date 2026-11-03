#!/usr/bin/env python3
"""Refresh checksums only after an intentional reviewed evidence edit; verification is verify.py."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parent
files={str(p.relative_to(root)):{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(root.rglob('*')) if p.is_file() and '__pycache__' not in p.parts and p.name!='manifest.json'}
record={'schema':1,'scope':'Preliminary held/analytic cloud geometry operator comparison; not selected lifecycle/source or runtime budget admission','file_count':len(files),'retained_bytes_excluding_manifest':sum(x['bytes'] for x in files.values()),'exact_builds':['81c3f43f76b490fa','173d261dc10b86c2','fd0181d8140a4ff9'],'dependency_pin':'../cloud-authored-masters-2026-11-03 at6f4d3fd075bc41153a62a88deb24c701af7725a5, density raw/compressed SHA pins in builds.json','files':files}
(root/'manifest.json').write_text(json.dumps(record,indent=2)+'\n');print('Retained manifest files:',len(files),'bytes excluding manifest:',record['retained_bytes_excluding_manifest'])
