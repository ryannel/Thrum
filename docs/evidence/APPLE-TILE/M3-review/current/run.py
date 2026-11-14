import sys, os, json, subprocess, hashlib, shutil
from pathlib import Path
from argparse import Namespace
root=Path.cwd(); out=root/'docs/evidence/APPLE-TILE/M3-review/current'
sys.path.insert(0,str(root/'scripts'))
from apple_tile_m0 import run_t2,finish_t2,ROUTE,save,digest
from exact_baselines import content_key
report={'revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'content_key':content_key(),'runs':{},'tests':{}}
args=Namespace(binary=root/'target/release/v1-game',store=out/'valley')
report['binary_sha256']=digest(args.binary)
save(out/'results.json',report)
def cmd(name,argv):
 with (out/(name+'.log')).open('w') as f:
  r=subprocess.run(argv,stdout=f,stderr=subprocess.STDOUT,env=dict(os.environ,RUSTC_WRAPPER=''))
 report['tests'][name]={'command':argv,'exit':r.returncode};save(out/'results.json',report)
 if r.returncode: raise RuntimeError(name)
for name,sel in [('input','synthetic::display::tests::input'),('timing','timing::tests'),('optional','synthetic::frame_stats::tests')]:
 cmd(name,['rustup','run','1.99.1','cargo','test','--locked','--offline','-p','thrum-render','--lib',sel,'--','--test-threads=1'])
cmd('cook',['target/debug/thrum-cook','cook','examples/v1-game/assets/showcase',str(args.store)])
route=json.loads(ROUTE.read_text());save(out/'route.json',dict(route,render_extent=[1920,1080]))
for recipe,case in [('portable','route'),('apple-tile','route'),('portable','variants'),('apple-tile','variants'),('apple-tile','render-stall')]:
 os.environ['THRUM_RECIPE']=recipe
 name=recipe+'-'+case; options={'mode':'realtime','case':case}
 if case=='route':options['route']=out/'route.json'
 report['runs'][name]=run_t2(args,out,name,options)
 report['runs'][name].update(finish_t2(out/name,viewing=True));save(out/'results.json',report)
 subprocess.run(['ffmpeg','-v','error','-i',str(out/name/'preview-1x.mp4'),'-vf','fps=10,scale=384:216,tile=5x8','-fps_mode','vfr',str(out/name/'inspection-%02d.png')],check=True)
 # All raw frames and selected PNGs were independently verified by finish_t2.
 m=json.loads((out/name/'manifest.json').read_text()); spool=out/name/m['spool_file'];report['runs'][name]['removed_verified_spool']={'file':spool.name,'bytes':spool.stat().st_size};spool.unlink();save(out/'results.json',report)
print(json.dumps({'passed':True,'runs':list(report['runs'])}))
