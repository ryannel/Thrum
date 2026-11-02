import json,sys
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection
import numpy as np
p=json.load(open(sys.argv[1]));w=p['probe']['feedTrajectories']
fig,axes=plt.subplots(1,2,figsize=(11,5),constrained_layout=True)
for i,t in enumerate(w['tracks']):
 if i%2 or t['sourceBirthTime']>240:continue
 a=np.array(t['samples']);
 for ax,columns,values in [(axes[0],[0,1],a[:,3]),(axes[1],[4,1],a[:,5])]:
  xy=a[:,columns]/np.array([1000 if columns[0]==0 else 1,1000]);segments=np.stack([xy[:-1],xy[1:]],axis=1)
  line=LineCollection(segments,cmap='viridis' if columns[0]==0 else 'coolwarm',norm=plt.Normalize(0,1),linewidth=1.6);line.set_array(values[:-1]);ax.add_collection(line);ax.autoscale_view()
axes[0].set(xlabel='World X (km)',ylabel='Height (km)',title='Approximate path; colour = sampled total q',ylim=(0,11))
axes[1].set(xlabel='Source time (s)',ylabel='Height (km)',title='Same path; colour = sampled ice fraction',ylim=(0,11))
for ax in axes:ax.axhline(9,color='gray',ls='--',lw=.7);ax.grid(alpha=.2)
fig.suptitle(f"Build {p['build']} · requested witness at {w['sourceTime']}s\nRK2 through frozen velocity, actual updated-q samples; not exact parcel lineage or a cloud beauty render")
fig.savefig(sys.argv[2],dpi=140)
