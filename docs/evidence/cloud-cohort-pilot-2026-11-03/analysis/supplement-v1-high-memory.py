from pathlib import Path
# Reuse only the diagnostic evaluator definition, before any large world-grid allocations.
p=Path(__file__).with_name('measure.py');exec(p.read_text().split('# Common weather-registered')[0])
data=json.loads((R/'supplement-state.json').read_text());base=json.loads((R/'metrics.json').read_text());lo=np.array(base['method']['world_grid_registered_min_corner_xyz_m']);dims=np.array(base['method']['world_grid_dimensions_xyz']);axes=[lo[i]+(np.arange(dims[i])+.5)*H for i in range(3)];out=[];imgs=[]
for snap in data['snapshots']:
 reg=np.array([snap['region'][0],0,snap['region'][2]]);cs=snap['items'];shape=tuple(dims[::-1]);sums=[np.zeros((dims[1],dims[0])),np.zeros((dims[2],dims[1]))];rootS=[a.copy()for a in sums];projs={c['id']:[np.zeros((dims[1],dims[0]),np.float32),np.zeros((dims[2],dims[1]),np.float32),np.zeros((dims[2],dims[0]),np.float32)]for c in cs};pots={c['id']:[a.copy()for a in projs[c['id']]]for c in cs};stats={c['id']:{str(t):[0,0,0]for t in thresholds}for c in cs if c['parentId']}
 for z in range(0,shape[0],4):
  zz=min(z+4,shape[0]);w=[axes[0][None,None,:]+reg[0],axes[1][None,:,None],axes[2][z:zz,None,None]+reg[2]];qs={c['id']:query(c,w)for c in cs};ps={c['id']:query(c,w,True)for c in cs};a=np.maximum.reduce(list(qs.values()));sums[0]+=a.sum(0)*H;sums[1][z:zz]=a.sum(2)*H;rootS[0]+=ps[1].sum(0)*H;rootS[1][z:zz]=ps[1].sum(2)*H
  for c in cs:
   for field,store in [(qs[c['id']],projs[c['id']]),(ps[c['id']],pots[c['id']])]:
    np.maximum(store[0],field.max(0),out=store[0]);store[1][z:zz]=field.max(2);store[2][z:zz]=field.max(1)
   if not c['parentId']:continue
   q=qs[c['id']];parent=ps[c['parentId']];old=np.maximum.reduce([ps[k['id']]for k in cs if k['born']<c['born']])
   for t in thresholds:
    m=q>t;v=stats[c['id']][str(t)];v[0]+=int(m.sum());v[1]+=int(np.count_nonzero(m&(parent<=t)));v[2]+=int(np.count_nonzero(m&(old<=t)))
 rows=[]
 for c in cs:
  if not c['parentId']:continue
  rr={'id':c['id'],'age':c['age'],'thresholds':{t:{'child_volume_m3':v[0]*H**3,'outside_parent_uneroded_fraction':v[1]/max(v[0],1),'outside_all_older_uneroded_fraction':v[2]/max(v[0],1)}for t,v in stats[c['id']].items()},'projection_vs_all_older':{}}
  for i,n in enumerate(['front_Z','side_X','top_Y']):
   q=projs[c['id']][i];old=np.maximum.reduce([pots[k['id']][i]for k in cs if k['born']<c['born']]);m=q>.005;nov=m&(old<=.005);dd=ndimage.distance_transform_edt(old<=.005)*H;rr['projection_vs_all_older'][n]={'novel_fraction_child':np.count_nonzero(nov)/max(np.count_nonzero(m),1),'max_distance_m':float(dd[nov].max(initial=0)),'area_ge32m_m2':int(np.count_nonzero(nov&(dd>=32)))*H**2}
  rows.append(rr)
 out.append({'time':snap['time'],'cohorts':rows});imgs.append((snap['time'],[1-np.exp(-.019*a)for a in [sums[0],rootS[0],sums[1].T,rootS[1].T]]));print('done',snap['time'],flush=True)
# Exact axis orientation: rowsY for both front and side.
W=330;HH=170;canvas=Image.new('RGB',(len(imgs)*W,4*HH),(15,20,27));draw=ImageDraw.Draw(canvas)
for j,(t,panels)in enumerate(imgs):
 for i,a in enumerate(panels):
  rgb=np.repeat(np.round(np.clip(a,0,1)*255).astype('uint8')[:,:,None],3,axis=2);pic=Image.fromarray(np.flipud(rgb));scale=min((W-12)/pic.width,(HH-28)/pic.height);pic=pic.resize((round(pic.width*scale),round(pic.height*scale)),Image.Resampling.NEAREST);canvas.paste(pic,(j*W+(W-pic.width)//2,i*HH+28));draw.text((j*W+5,i*HH+5),f't{t} '+['actual front','root potential front','actual side','root potential side'][i],fill='white')
canvas.save(R/'registered-opacity-correct-axes.png')
# Candidate compact crown masks; content feasibility only, not certified or adopted.
crops=[]
for name,low,high in [('crownA',[208,232,52],[368,344,220]),('crownB',[116,232,156],[284,344,292])]:
 z,y,x=np.ogrid[:256,:256,:256];a=[2*x+1,2*y+1,2*z+1];mask=1.
 for k in range(3):mask=mask*smooth((a[k]-low[k])/12)*smooth((high[k]-a[k])/12)
 q=rho*mask;inside=q>.041;edt=ndimage.distance_transform_edt(inside)*2;rootband=(a[1]>=248)&(a[1]<=278);idx=np.unravel_index(np.argmax(edt*rootband),edt.shape);root=(np.array(idx)[::-1]*2+1).tolist();radius=float(edt[idx]);pts=np.argwhere(inside);bbox=[(pts.min(0)[::-1]*2).tolist(),((pts.max(0)[::-1]+1)*2).tolist()];lab,n=ndimage.label(inside);rootid=lab[idx];connected=int(np.count_nonzero(lab==rootid));crops.append({'candidate':name,'roi_local_xyz':[low,high],'edge_taper_local_m':12,'q_attach':.04,'numerical_threshold':.041,'candidate_root_local_xyz':root,'native_EDT_radius_local_m':radius,'actual_lower_bound_radius_NOT_CERTIFIED':'EDT centre-distance requires interpolation guard and post-distance-taper witness; old wholemaster distance invalid','threshold_bbox_local_xyz':bbox,'positive_root_component_fraction':connected/max(int(inside.sum()),1),'scope':'masked density candidate only; no retained/cooked/adopted rho or distance'})
(R/'supplement.json').write_text(json.dumps({'sampling':'8m world; scalar moments streaming; side image axes corrected','snapshots':out,'candidate_component_screen':crops,'peak_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss},indent=2)+'\n');print('ALLDONE',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,flush=True)
