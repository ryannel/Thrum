import json,subprocess,time,sys,hashlib
from pathlib import Path
root=Path('tmp/g3t-m2/attended/visible');recipe,name=sys.argv[1:]
ws=json.loads(subprocess.check_output(['tmp/g3t-m2/attended/windows']))
w=next(w for w in ws if 'Thrum V1' in w.get('kCGWindowName',''))
subprocess.run(['/usr/sbin/screencapture','-x','-o','-l',str(w['kCGWindowNumber']),str(root/(recipe+'-'+name+'.png'))],check=True)
row={'unix_ns':time.time_ns(),'monotonic_ns':time.monotonic_ns(),'recipe':recipe,'window':w,'binary_sha256':hashlib.sha256(Path('tmp/g3t-m2/attended/ThrumM2.app/Contents/MacOS/v1-game').read_bytes()).hexdigest(),'controls':json.loads((root/(recipe+'-controls.json')).read_text())}
(root/(recipe+'-'+name+'.json')).write_text(json.dumps(row,indent=2)+'\n')
print(json.dumps(row,indent=2))
