import subprocess,json,sys,time,hashlib,ctypes,re
from pathlib import Path
recipe,name=sys.argv[1:];root=Path('tmp/g3t-m3-mf1/interactive');root.mkdir(exist_ok=True)
ws=json.loads(subprocess.check_output(['tmp/g3t-m2/attended/windows']))
w=next(w for w in ws if 'Thrum V1' in w.get('kCGWindowName','') and w.get('kCGWindowOwnerName')=='ThrumM2')
pid=w['kCGWindowOwnerPID'];lib=ctypes.CDLL('/usr/lib/libproc.dylib');buf=ctypes.create_string_buffer(4096);assert lib.proc_pidpath(pid,buf,len(buf))>0
binary=Path(buf.value.decode());expected=Path('tmp/g3t-m3-mf1/ThrumM2.app/Contents/MacOS/v1-game').resolve();assert binary==expected,(binary,expected)
process=subprocess.check_output(['ps','eww','-p',str(pid),'-o','command='],text=True)
actual_recipe=re.search(r'(?:^|\s)THRUM_RECIPE=([^\s]+)',process)[1];assert actual_recipe==recipe,(actual_recipe,recipe)
start=time.monotonic_ns();subprocess.run(['/usr/sbin/screencapture','-x','-o','-l',str(w['kCGWindowNumber']),str(root/f'{recipe}-{name}.png')],check=True)
stream=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-of','json',str(root/f'{recipe}-{name}.png')]))['streams'][0]
row={'unix_ns':time.time_ns(),'monotonic_before_ns':start,'monotonic_after_ns':time.monotonic_ns(),'recipe':recipe,'observed_process_recipe':actual_recipe,'window':w,'pid':pid,'loaded_executable_path':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'controls':json.loads((root/(recipe+'-controls.json')).read_text()),'image_extent':[stream['width'],stream['height']],'backing_scale':2,'content_bounds_pixels':[0,64,stream['width'],stream['height']],'reference':f'window-references/{recipe}-noon/checkpoint-0.png','scope':'Matched paused controls/content/grid, not the same submission or exact compositor bytes'}
(root/f'{recipe}-{name}.json').write_text(json.dumps(row,indent=2)+'\n');print({k:row[k] for k in ['pid','recipe','binary_sha256','image_extent','content_bounds_pixels']})
