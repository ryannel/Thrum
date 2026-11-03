#!/usr/bin/env python3
"""Relocate this evidence, restore all three exact builds, then run their scoped tests."""
from pathlib import Path
import argparse,json,shutil,subprocess,sys,tempfile
ROOT=Path(__file__).resolve().parent
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--master-package',type=Path,default=ROOT.parent/'cloud-authored-masters-2026-11-03');a=p.parse_args();results=[]
with tempfile.TemporaryDirectory(prefix='cloud-geometry-recovery-') as td:
 base=Path(td);relocated=base/'relocated-evidence';shutil.copytree(ROOT,relocated,ignore=shutil.ignore_patterns('__pycache__'));output=base/'restored'
 subprocess.run([sys.executable,str(relocated/'restore.py'),'--build','all','--output',str(output),'--master-package',str(a.master_package.resolve())],check=True)
 for build in json.loads((relocated/'builds.json').read_text())['builds']:
  root=output/build;namespace={'__file__':str(root/'serve.py'),'__name__':'restored_serve'};exec(compile((root/'serve.py').read_text(),'serve.py','exec'),namespace);_,actual=namespace['payload']();expected=json.loads((relocated/'identities'/f'{build}.json').read_text());assert actual==expected
  tests=sorted((root/'tests').glob('*.test.mjs'));logs=[]
  for test in tests:
   result=subprocess.run(['node',str(test)],text=True,capture_output=True,check=True);logs.append({'test':str(test.relative_to(root)),'stdout':result.stdout.strip(),'exit':result.returncode})
  results.append({'build':build,'served_file_count':len(actual['files']),'all_exact_payload_hashes':True,'actual_server_payload_identity':True,'relocated_tests':logs})
 print(json.dumps({'scope':'Clean temporary relocation using only retained evidence and referenced committed master gzip; no live tmp input, browser/GPU/encoder execution or recook. Temporary trees removed after this run.','temporary_logical_bytes_before_cleanup':sum(p.stat().st_size for p in base.rglob('*') if p.is_file()),'retained_package_logical_bytes_at_run':sum(p.stat().st_size for p in ROOT.rglob('*') if p.is_file()),'builds':results},indent=2))
