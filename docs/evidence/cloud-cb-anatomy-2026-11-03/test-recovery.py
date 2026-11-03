from pathlib import Path
import importlib.util,json,tempfile
ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('restore',ROOT/'restore.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
results=[]
for build in ['f30d926dcc6695fc','0ab02b3a38d03427']:
 with tempfile.TemporaryDirectory(prefix='thrum-cb-recovery-',dir='/private/tmp') as tmp:
  results.append(m.restore(build,Path(tmp)/'instrument',True))
(ROOT/'recovery.json').write_text(json.dumps({'scope':'Exact archive/source recovery and retained Node tests, no browser/GPU/art admission','one_build_at_a_time':True,'temporary_outputs_removed':True,'results':results},indent=2)+'\n')
print(json.dumps(results,indent=2))
