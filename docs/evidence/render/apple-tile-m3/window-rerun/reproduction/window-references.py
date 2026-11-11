import sys,os,json
from pathlib import Path
from argparse import Namespace
sys.path.insert(0,'scripts')
from apple_tile_m0 import run_t2,finish_t2,save
root=Path('tmp/g3t-m3-mf1/window-references').resolve();root.mkdir(exist_ok=True)
args=Namespace(binary=Path('tmp/g3t-m3-mf1/bin/v1-game').resolve(),store=Path('tmp/g3t-m0/valley').resolve())
os.environ['THRUM_ROUTE_OVERLAY']='proof'
report=json.loads((root/'report.json').read_text()) if (root/'report.json').exists() else {}
for recipe in ['portable','apple-tile']:
 os.environ['THRUM_RECIPE']=recipe
 for name,params in [('sun','showcase-sunset-clear-sun'),('night','showcase-night-upslope'),('noon','showcase-noon')]:
  key=recipe+'-'+name
  if key in report and report[key].get('complete'):continue
  report[key]=run_t2(args,root,key,{'mode':'still','params':Path('examples/v1-game/params')/(params+'.params.json')})
  report[key].update(finish_t2(root/key))
  save(root/'report.json',report)
print('Matched paused-world still references ready.')
