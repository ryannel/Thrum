from pathlib import Path
import hashlib,json,subprocess,time,os,sys
R=Path(__file__).resolve().parent;PY='/opt/homebrew/bin/python3.14';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(R/'COMMISSION.md')=='e482b4c40ab5324054a429717f801b5089295bfc6e525f8480552dcc191d3dc3'
source=R.parent/'cloud-natural-crown-v1/natural-pre-distance.r16f';assert sha(source)=='6cf3de4e99ea6a5f79d32e1506e9375aadd1aaef6f770615ecd2b8341537ee09'
if not(R/'density.r16f').exists():os.link(source,R/'density.r16f')
rows=[]
for stage in sys.argv[1:]or['inside','outside','merge','radius','certs']:
 start=time.perf_counter();peak=0;failure=None
 with(R/(stage+'.log')).open('w')as f:
  p=subprocess.Popen([PY,str(R/'cook.py'),stage],stdout=f,stderr=subprocess.STDOUT)
  while p.poll() is None:
   v=subprocess.run(['ps','-o','rss=','-p',str(p.pid)],capture_output=True,text=True).stdout.strip()
   if v:
    rss=int(v)*1024;peak=max(peak,rss)
    if rss>=112*2**20:failure={'reason':'earlyRSS','rss_bytes':rss,'hard160_exceeded':rss>=160*2**20};p.terminate();break
   time.sleep(.02)
  p.wait()
 row={'stage':stage,'exitCode':p.returncode,'sampled_peak_rss_bytes':peak,'elapsed_s':time.perf_counter()-start,'failure':failure,'recipe_sha256':sha(R/'cook.py')};rows.append(row);(R/(stage+'-run.json')).write_text(json.dumps(row,indent=2)+'\n');print(json.dumps(row),flush=True)
 if p.returncode or failure:raise SystemExit('STOP: inspect '+stage+'.log before further work')
