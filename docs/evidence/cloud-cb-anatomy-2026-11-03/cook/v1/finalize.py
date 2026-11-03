from pathlib import Path
import numpy as np,json,hashlib,platform,resource
R=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def projections(side):
 q=np.memmap(R/f'cb-anatomy-{side}.density.r16f',dtype='<f2',mode='r',shape=(side,side,side));p=[np.zeros((side,side),np.float64) for _ in range(3)];s=0.;peak=0.;nonzero=0;invalid=0
 for z in range(0,side,8):
  a=q[z:z+8].astype(np.float32);p[0]+=a.sum(0,dtype=np.float64);p[1][z:z+len(a)]=a.sum(2,dtype=np.float64);p[2][z:z+len(a)]=a.sum(1,dtype=np.float64);s+=float(a.sum(dtype=np.float64));peak=max(peak,float(a.max()));nonzero+=int(np.count_nonzero(a));invalid+=int(np.count_nonzero(~np.isfinite(a)|(a<0)|(a>1)))
 return p,{'sum':s,'peak':peak,'nonzero':nonzero,'invalid':invalid,'zero_faces':all(not np.any(f) for f in [q[0],q[-1],q[:,0],q[:,-1],q[:,:,0],q[:,:,-1]])}
def avg(a):
 i=np.arange(256);j=np.floor(i*1.5).astype(int);w=np.where(i%2==0,2/3,1/3);b=a[j,:]*w[:,None]+a[j+1,:]*(1-w[:,None]);return b[:,j]*w[None,:]+b[:,j+1]*(1-w[None,:])
f,fs=projections(384);c,cs=projections(256);pitch384=[12000/384,14000/384,12000/384];pitch256=[12000/256,14000/256,12000/256];metrics={}
for axis,name in enumerate(['front_alongZ','side_alongX','top_alongY']):
 world_axis=[2,0,1][axis];t=f[axis]*pitch384[world_axis]*.0045;tc=c[axis]*pitch256[world_axis]*.0045;targetT=avg(np.exp(-t));actualT=np.exp(-tc);metrics[name]={'max_abs_tau_error_vs_native_footprint_average':float(np.max(np.abs(tc-avg(t)))),'mean_abs_tau_error':float(np.mean(np.abs(tc-avg(t)))),'max_abs_T_error_vs_native_footprint_average':float(np.max(np.abs(actualT-targetT))),'mean_abs_T_error':float(np.mean(np.abs(actualT-targetT)))}
record={'native_side':384,'comparison_side':256,'downsample':'Exact separable overlap average of384piecewise-constant cells, thenbinary16; not centre sampling','kappa_per_m':.0045,'relative_density_integral_error':cs['sum']*(384/256)**3/fs['sum']-1,'coarse_to_native_peak_ratio':cs['peak']/fs['peak'],'radiographic_comparison':metrics,'scope':'Three orthographic footprint-average optical-depth/transmittance comparisons; not arbitrary-ray/beauty alias qualification','recipe_sha256':sha(Path(__file__)),'python':platform.python_version(),'numpy':np.__version__}
(R/'resolution-diagnostics.json').write_text(json.dumps(record,indent=2)+'\n')
source=json.loads((R/'cb-anatomy-384.json').read_text());source['source_inputs']=json.loads((R/'inputs.json').read_text());source['resolution_diagnostics']={'file':'resolution-diagnostics.json','sha256':sha(R/'resolution-diagnostics.json')};source['postprocess_recipe']={'file':'finalize.py','sha256':sha(Path(__file__)),'peak_process_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss};(R/'cb-anatomy-384.json').write_text(json.dumps(source,indent=2)+'\n')
coarse=json.loads((R/'cb-anatomy-256.json').read_text());coarse['stats']=cs;coarse['source_inputs']=source['source_inputs'];coarse['postprocess_recipe']=source['postprocess_recipe'];coarse['resolution_diagnostics']=source['resolution_diagnostics'];coarse['native_reference_metadata']={'file':'cb-anatomy-384.json','sha256':sha(R/'cb-anatomy-384.json'),'scope':'384native attachment/topology statistics apply to that reference, not256'};(R/'cb-anatomy-256.json').write_text(json.dumps(coarse,indent=2)+'\n');print(json.dumps(record,indent=2))
