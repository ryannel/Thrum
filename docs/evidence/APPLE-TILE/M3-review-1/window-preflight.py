import ctypes,json,subprocess,time,sys,os
from pathlib import Path
cg=ctypes.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics');cg.CGPreflightScreenCaptureAccess.restype=ctypes.c_bool
x={'screen_recording_granted':cg.CGPreflightScreenCaptureAccess(),'console_locked':'"IOConsoleLocked" = Yes' in subprocess.check_output(['/usr/sbin/ioreg','-n','Root','-d1'],text=True),'unix_ns':time.time_ns(),'monotonic_ns':time.monotonic_ns(),'grant_requested':False,'probe_pid':os.getpid(),'parent_pid':os.getppid(),'probe_executable':sys.executable,'api':'CGPreflightScreenCaptureAccess','host':subprocess.check_output(['sw_vers'],text=True),'observer_planned':'/usr/sbin/screencapture'}
Path(__file__).with_name('preflight-device-access.json').write_text(json.dumps(x,indent=2)+'\n');print(json.dumps(x))
