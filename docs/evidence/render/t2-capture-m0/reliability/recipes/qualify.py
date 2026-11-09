import subprocess,json,os,fcntl,time,hashlib,shutil
from pathlib import Path
root=Path('tmp/t2-capture-m0-r3/qualified');root.mkdir();rows=[]
def save(): (root/'order.json').write_text(json.dumps(rows,indent=2)+'\n')
def session(name,case,limit=None,active=False,mode='realtime',profile=False,expect=0):
 out=root/name;env=dict(os.environ,THRUM_CAPTURE_PACING='active' if active else 'sleep',THRUM_CAPTURE_QOS='interactive')
 if limit:env['THRUM_CAPTURE_DISK_BPS']=str(limit)
 cmd=['python3','scripts/capture_v1_runtime.py','--binary','target/release/v1-game','--output',str(out),'--case',case,'--mode',mode,'--defer']
 if case not in ('running','writer-slow'):cmd+=['--store','tmp/t2-capture-m0/valley']
 if profile:cmd+=['--profile']
 row={'name':name,'case':case,'command':cmd,'active':active,'limit_Bps':limit,'expected_exit':expect,'start_ns':time.monotonic_ns()};rows.append(row);save()
 with (root/(name+'.log')).open('w') as stream:row['exit']=subprocess.run(cmd,env=env,stdout=stream,stderr=subprocess.STDOUT).returncode
 row['end_ns']=time.monotonic_ns();save();print(name,row['exit'],flush=True)
 if row['exit']!=expect:raise RuntimeError(row)
for i in range(5):
 out=root/f'writer-{i:02}';out.mkdir();cmd=['target/release/examples/capture_spool'];row={'name':out.name,'command':cmd,'source':'tmp/t2-capture-m0-rework/integrated/pan-1/source-lossless.mkv','source_sha256':None};rows.append(row);save()
 with (out/'frames.zst').open('wb',buffering=0) as stream,(out/'result.txt').open('wb') as errors:
  fcntl.fcntl(stream.fileno(),48,1)
  decoder=subprocess.Popen(['ffmpeg','-v','error','-i',row['source'],'-frames:v','480','-f','rawvideo','-pix_fmt','rgba','pipe:1'],stdout=subprocess.PIPE)
  result=subprocess.run(cmd,stdin=decoder.stdout,stdout=stream,stderr=errors);decoder.stdout.close();assert decoder.wait()==0
  begin=time.monotonic_ns();os.fsync(stream.fileno());fcntl.fcntl(stream.fileno(),51);row['full_flush_ns']=time.monotonic_ns()-begin
 row['exit']=result.returncode;save();print(out.name,row['exit'],flush=True);assert result.returncode==0
for i in range(30):session(f'series-{i:02}', ['pan','input-stall','running'][i%3],None if i<10 else 200_000_000)
for i,case in enumerate(['off','pan','pan','off']*2):session(f'cost-{i:02}',case,active=True)
for mode in ['still','fixed']:session(mode,'pan',mode=mode)
session('render-stall','render-stall')
session('writer-slow','writer-slow',expect=1)
for case in ['off','pan']:session('profile-'+case,case,active=True,profile=True)
(root/'binary.sha256').write_text(hashlib.sha256(Path('target/release/v1-game').read_bytes()).hexdigest()+'\n')
print('RECORDING COMPLETE',flush=True)
