from pathlib import Path
code=Path('tmp/t2-capture-m0-r3/qualify.py').read_text().split('for i in range(5):')[0].replace('root.mkdir();rows=[]',"rows=json.loads((root/'order.json').read_text())")
exec(code)
rows[-1]['control_outcome']='Unexpectedly complete: compressed diagnostic grid fits encoded reserve; retained, not counted as a saturation proof.';save()
os.environ['THRUM_CAPTURE_ENCODING']='raw'
session('writer-slow-raw','writer-slow',expect=1)
del os.environ['THRUM_CAPTURE_ENCODING']
for case in ['off','pan']:session('profile-'+case,case,active=True,profile=True)
(root/'binary.sha256').write_text(hashlib.sha256(Path('target/release/v1-game').read_bytes()).hexdigest()+'\n')
print('RECORDING COMPLETE',flush=True)
