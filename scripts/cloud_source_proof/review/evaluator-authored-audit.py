"""Read-only continuous authored-source math and component audit."""
import importlib.util, hashlib, json
from pathlib import Path
import numpy as np
from scipy import ndimage
from scipy.spatial import cKDTree

ROOT=Path(__file__).resolve().parents[3]
# script is <repo>/tmp/cloud-source-proof/review/evaluator-authored-audit.py
def load_source():
    path=ROOT/"scripts/cloud_source_proof/authored.py"
    spec=importlib.util.spec_from_file_location("reviewed_source",path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return path,module,module.Source()

def describe(field,threshold,connectivity):
    mask=field>threshold
    labels,count=ndimage.label(mask,structure=ndimage.generate_binary_structure(3,connectivity))
    sizes=np.bincount(labels.ravel());sizes[0]=0
    ids=np.argsort(sizes)[::-1][:8]
    ids=ids[sizes[ids]>0]
    result=[]
    substantial=[]
    for ident in ids:
        points=np.argwhere(labels==ident)
        lo=points.min(axis=0);hi=points.max(axis=0)
        sl=tuple(slice(int(a),int(b)+1) for a,b in zip(lo,hi))
        component=np.where(labels[sl]==ident,field[sl],0).astype(np.float64)
        taus=[float(component.sum(axis=a).max()*4.) for a in [2,1,0]]
        entry={"label":int(ident),"voxels":int(sizes[ident]),
            "index_bbox_min_zyx":lo.tolist(),"index_bbox_max_zyx":hi.tolist(),
            "max_sigma_per_m":float(component.max()),"max_axial_tau_xyz":taus,
            "integrated_extinction_volume_m2":float(component.sum()*64)}
        result.append(entry)
        if max(taus)>=2:substantial.append((entry,points))
    gaps=[]
    for i in range(min(len(substantial),4)):
        for j in range(i+1,min(len(substantial),4)):
            a,pa=substantial[i];b,pb=substantial[j]
            tree=cKDTree(pa.astype(np.float64)*4.)
            distances,nearest=tree.query(pb.astype(np.float64)*4.,k=1)
            k=int(np.argmin(distances));d=float(distances[k])
            p0=pa[int(nearest[k])].astype(np.float64);p1=pb[k].astype(np.float64)
            n=max(3,int(np.ceil(d/.125))+1)
            coords=np.linspace(p0,p1,n).T
            sampled=ndimage.map_coordinates(field,coords,order=1,mode="constant",cval=0,prefilter=False)
            zero=sampled==0
            longest=run=0
            for value in zero:
                run=run+1 if value else 0;longest=max(longest,run)
            gaps.append({"labels":[a["label"],b["label"]],
                "nearest_voxel_centre_distance_m":d,
                "conservative_voxel_cube_separation_lower_m":max(0.,d-4*np.sqrt(3)),
                "trilinear_positive_support_separation_lower_m":max(0.,d-8*np.sqrt(3)) if connectivity==3 and threshold==0 else None,
                "zero_interpolated_samples_on_nearest_centre_segment":int(zero.sum()),
                "longest_sampled_zero_run_m":max(0,longest-1)*d/(n-1),
                "segment_sample_spacing_m":d/(n-1)})
    return {"threshold_sigma_per_m":threshold,
        "connectivity":"6-neighbour" if connectivity==1 else "26-neighbour",
        "component_count":int(count),"positive_voxels":int(mask.sum()),
        "largest_eight":result,"substantial_pairs_among_largest_four":gaps}

def main():
    path,module,source=load_source()
    report={"recipe_sha256":hashlib.sha256(path.read_bytes()).hexdigest(),
        "shape_sha256":source.shape_sha256,"descriptor":source.descriptor(),
        "scope":"Actual source.field at voxel centres; independent component/optical diagnostics. Not motion playback or runtime proof.",
        "smooth_sample_count":100001,"ages":{}}
    smooth=module.smooth(np.linspace(-.1,1.1,100001))
    report["smooth"]={"minimum":float(smooth.min()),"maximum":float(smooth.max()),
        "monotone":bool(np.all(np.diff(smooth)>=0)),"first":float(smooth[0]),"last":float(smooth[-1])}
    report["source_fields"]={"birth_min_s":float(source.birth.min()),"birth_max_s":float(source.birth.max()),
        "death_min_s":float(source.death.min()),"death_max_s":float(source.death.max()),
        "base_max_sigma":float(source.base.max())}
    endpoint_report={}
    for age in [0.,20.,50.,60.,67.5,90.,140.,165.,185.,200.,215.,240.]:
        field=source.field(age)
        faces=[field[:8],field[-8:],field[:,:8],field[:,-8:],field[:,:,:8],field[:,:,-8:]]
        endpoint_report[str(age)]={"min":float(field.min()),"max":float(field.max()),
            "positive_voxels":int(np.count_nonzero(field)),
            "collar_max_sigma":max(float(f.max()) for f in faces),
            "integrated_extinction_volume_m2":float(field.astype(np.float64).sum()*64.)}
        if age in [90.,165.,185.,200.,215.]:
            report["ages"][str(age)]=[describe(field,t,c) for c in [1,3] for t in [0.,1e-6,.001,.01]]
    report["sampled_field_bounds"]=endpoint_report
    # Compare whole-field scalar-gain null on two actual lifecycle intervals.
    gain=[]
    for a,b in [(50.,90.),(165.,185.),(185.,200.)]:
        fa=source.field(a).astype(np.float64);fb=source.field(b).astype(np.float64)
        g=max(0.,float(np.vdot(fa,fb)/np.vdot(fa,fa)))
        gain.append({"ages_s":[a,b],"best_global_gain":g,
            "relative_l2_residual":float(np.linalg.norm(fb-g*fa)/np.linalg.norm(fb))})
    report["global_gain_null"]=gain
    images=ROOT/"tmp/cloud-source-proof/authored-erosion-screen"
    report["image_hashes"]={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(images.glob("*.png"))}
    output=ROOT/"tmp/cloud-source-proof/review/evaluator-authored-audit.json"
    output.write_text(json.dumps(report,indent=2)+"\n")
    print("REPORT",output)
    print("RECIPE",report["recipe_sha256"])
    print("BOUNDS",json.dumps(endpoint_report))
    for age in [90.,165.,185.,200.,215.]:
        entries=report["ages"][str(age)]
        print("AGE",age,json.dumps([{"threshold":r["threshold_sigma_per_m"],"connectivity":r["connectivity"],
            "components":r["component_count"],"substantial":sum(max(v["max_axial_tau_xyz"])>=2 for v in r["largest_eight"]),
            "sizes":[v["voxels"] for v in r["largest_eight"][:4]],"pairs":r["substantial_pairs_among_largest_four"]} for r in entries]))
if __name__=="__main__":main()
