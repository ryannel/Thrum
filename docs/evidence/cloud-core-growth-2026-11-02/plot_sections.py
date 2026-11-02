import json,sys
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Ellipse
p=json.load(open(sys.argv[1]));s=p['probe']['morphology']['sections']; h=s['head']; b=s['bounds']
fig,(ax,profile)=plt.subplots(1,2,figsize=(10,6),gridspec_kw={'width_ratios':[3,1]},constrained_layout=True)
a=s['xy'];v=np.array(a['values']).reshape(a['dims'][1],a['dims'][0])
im=ax.imshow(v,origin='lower',extent=[b[0][0]/1000,b[1][0]/1000,b[0][1]/1000,b[1][1]/1000],interpolation='nearest',cmap='magma',vmin=0,vmax=1,aspect='equal')
ax.add_patch(Ellipse((h[0]/1000,h[1]/1000),2*h[3]/1000,2*h[5]/1000,fill=False,color='cyan',lw=1,label='control R/H'))
ax.set(xlim=((h[0]-2*h[3])/1000,(h[0]+2*h[3])/1000),ylim=(max(.5,(h[1]-3*h[5])/1000),(h[1]+2*h[5])/1000),xlabel='World X (km)',ylabel='Height (km)',title=f"Native q, Z voxel {a['fixedZ']}; control is outline only")
profile.plot([v[1] for v in s['axis']],[v[0]/1000 for v in s['axis']]);profile.set(ylim=ax.get_ylim(),xlim=(0,1),xlabel='q on head axis',title='Native column');profile.grid(alpha=.25)
fig.colorbar(im,ax=ax,label='Liquid + ice optical proxy')
fig.suptitle(f"Build {p['build']} · source {p['probe']['time']:.2f}s · head {int(h[8])}\nNo material, light or image reconstruction; nearest native voxels")
fig.savefig(sys.argv[2],dpi=150);print(matplotlib.__version__,sys.argv[2])
