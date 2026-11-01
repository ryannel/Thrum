"""Independent ranked-byte decoder and current-source quarter-time replay.
Does not import cooker, direct.py, ranked.py or candidate reader.
"""
import hashlib, importlib.util, json, struct, time
from pathlib import Path
import numpy as np

ROOT=Path(__file__).resolve().parents[3]
BASE=ROOT/"tmp/cloud-source-proof"
def sha(data):return hashlib.sha256(data).hexdigest()
def decode(data,record,ranked):
    start=int(record["offset"]);length=int(record["bytes"])
    frame=memoryview(data)[start:start+length]
    assert len(frame)==length and sha(frame)==record["sha256"]
    index_bytes=int(record["index_bytes"]);occupancy_bytes=int(record["occupancy_bytes"])
    ids=np.frombuffer(frame[:index_bytes],dtype="<u2")
    assert len(ids)==4096 and occupancy_bytes==512
    occupancy=np.unpackbits(np.frombuffer(frame[index_bytes:index_bytes+occupancy_bytes],np.uint8),bitorder="little")
    assert np.array_equal(occupancy,ids!=65535)
    active=int(record["active_bricks"])
    assert np.array_equal(np.sort(ids[ids!=65535]),np.arange(active))
    header=index_bytes+occupancy_bytes
    pointers=None
    if ranked:
        offset=int(record["pointer_table_offset"])
        assert offset==header and int(record["pointer_table_bytes"])==active*4
        pointers=np.frombuffer(frame[offset:offset+active*4],dtype="<u4")
        assert int(record["brick_data_offset"])>=offset+active*4
    else:assert length==header+active*512
    grid=np.zeros((128,128,128),np.uint8)
    counts={"dense":0,"ranked":0}
    for brick_location,brick_id in enumerate(ids):
        if brick_id==65535:continue
        brick_id=int(brick_id)
        if not ranked:
            offset=header+512*brick_id
            local=np.frombuffer(frame[offset:offset+512],np.uint8)
            counts["dense"]+=1
        else:
            tagged=int(pointers[brick_id]);offset=tagged&~1
            assert offset%4==0 and offset>=int(record["brick_data_offset"])
            if not(tagged&1):
                assert offset+512<=length
                local=np.frombuffer(frame[offset:offset+512],np.uint8)
                counts["dense"]+=1
            else:
                assert offset+80<=length
                masks=np.frombuffer(frame[offset:offset+64],dtype="<u4")
                prefix=np.frombuffer(frame[offset+64:offset+80],dtype="<u2")
                # Independent decoder assigns ascending mask positions directly,
                # rather than using the runtime prefix/popcount query algorithm.
                bits=np.unpackbits(np.frombuffer(frame[offset:offset+64],np.uint8),bitorder="little")
                positions=np.flatnonzero(bits)
                nz=len(positions)
                assert 80+((nz+3)//4)*4<512
                expected=np.array([np.count_nonzero(bits[:k*64]) for k in range(8)])
                assert np.array_equal(prefix,expected)
                assert offset+80+((nz+3)//4)*4<=length
                values=np.frombuffer(frame[offset+80:offset+80+nz],np.uint8)
                assert np.all(values>0)
                local=np.zeros(512,np.uint8);local[positions]=values
                counts["ranked"]+=1
        bz,rem=divmod(brick_location,256);by,bx=divmod(rem,16)
        grid[bz*8:bz*8+8,by*8:by*8+8,bx*8:bx*8+8]=local.reshape(8,8,8)
    assert sha(grid.tobytes())==record["quantized_frame_sha256"]
    return grid,counts

def optical_error(candidate,reference):
    tau=trans=0.
    for axis in range(3):
        ca=candidate.sum(axis=axis,dtype=np.float64)*4.
        re=reference.sum(axis=axis,dtype=np.float64)*4.
        tau=max(tau,float(np.max(np.abs(ca-re))))
        trans=max(trans,float(np.max(np.abs(np.exp(-ca)-np.exp(-re)))))
    return {"max_axial_tau_error":tau,"max_axial_T_abs_error":trans}

def main():
    begin=time.perf_counter()
    olddir=BASE/"authored-cooked-v2";newdir=BASE/"authored-ranked"
    oldjson=(olddir/"cloudclip.json").read_bytes();newjson=(newdir/"cloudclip.json").read_bytes()
    old=json.loads(oldjson);new=json.loads(newjson)
    olddata=(olddir/"density.bin").read_bytes();newdata=(newdir/"density.bin").read_bytes()
    olddiag=(olddir/"diagnostics.json").read_bytes();newdiag=(newdir/"diagnostics.json").read_bytes()
    assert olddiag==newdiag
    diagnostic=json.loads(newdiag)
    assert old["source_times_s"]==new["source_times_s"]
    times=np.array(new["source_times_s"],np.float64)
    assert len(times)==162 and times[0]==0 and times[-1]==240 and np.all(np.diff(times)>0)
    assert len(new["density_frames"])==len(old["density_frames"])==len(times)
    assert new["source_descriptor"]==old["source_descriptor"]
    sourcepath=ROOT/"scripts/cloud_source_proof/authored.py"
    sourcehash=sha(sourcepath.read_bytes())
    assert sourcehash==new["source_descriptor"]["source_recipe_sha256"]
    spec=importlib.util.spec_from_file_location("reviewed_actual_source",sourcepath)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    source=module.Source()
    support=np.argwhere(source.base>0)
    lo=support.min(axis=0);hi=support.max(axis=0)
    roi=tuple(slice(int(a),int(b)+1) for a,b in zip(lo,hi))
    scale=float(new["runtime_sigma_max_per_m"])/255.
    endpoints=[];counts={"dense":0,"ranked":0};frame_hashes=[];endpoint_errors=[]
    # Only cropped endpoint grids survive. All full voxel grids are independently
    # decoded and compared, including exact zero outside this fixed support ROI.
    source_evaluations=0
    for i,(ro,rn) in enumerate(zip(old["density_frames"],new["density_frames"])):
        go,_=decode(olddata,ro,False);gn,c=decode(newdata,rn,True)
        assert np.array_equal(go,gn)
        for k in counts:counts[k]+=c[k]
        assert not np.any(gn[source.base==0])
        endpoints.append(gn[roi].copy())
        frame_hashes.append(sha(gn.tobytes()))
        ref=source.field(float(times[i]))[roi].astype(np.float64);source_evaluations+=1
        values=gn[roi].astype(np.float64)*scale
        endpoint_errors.append({"source_time_s":float(times[i]),**optical_error(values,ref)})
    leaf_receipts=diagnostic["accepted_leaf_quarter_checks"]
    assert len(leaf_receipts)==len(times)-1==161
    assert not diagnostic["unresolved_minimum_leaves"]
    quarter_checks=[];half_failures=[];pre_failures=[]
    for i,leaf in enumerate(leaf_receipts):
        a,b=times[i:i+2]
        assert leaf["start_s"]==a and leaf["end_s"]==b
        actual_receipt_times=[v["source_time_s"] for v in leaf["checks"]]
        assert actual_receipt_times==[float(a+(b-a)*theta) for theta in [.25,.5,.75]]
        for theta in [.25,.5,.75]:
            t=float(a+(b-a)*theta)
            ref=source.field(t)[roi].astype(np.float64);source_evaluations+=1
            # Emulate the direct scalar field build's float32 endpoint dequant
            # and linear interpolation, then its actual R16F current-field store.
            av=endpoints[i].astype(np.float32)*np.float32(scale)
            bv=endpoints[i+1].astype(np.float32)*np.float32(scale)
            candidate=np.float32(1-theta)*av+np.float32(theta)*bv
            pre=optical_error(candidate.astype(np.float64),ref)
            half=optical_error(candidate.astype(np.float16).astype(np.float64),ref)
            record={"source_time_s":t,"start_s":float(a),"end_s":float(b),"pre_half":pre,"R16F":half}
            quarter_checks.append(record)
            if pre["max_axial_tau_error"]>.1 or pre["max_axial_T_abs_error"]>.02:pre_failures.append(record)
            if half["max_axial_tau_error"]>.1 or half["max_axial_T_abs_error"]>.02:half_failures.append(record)
    assert sha(sourcepath.read_bytes())==sourcehash
    admitted=len(newdata)+len(newjson)
    def maxima(key):
        return {"max_axial_tau_error":max(r[key]["max_axial_tau_error"] for r in quarter_checks),
            "max_axial_T_abs_error":max(r[key]["max_axial_T_abs_error"] for r in quarter_checks)}
    report={"scope":"Independent complete ranked/dense byte decode equivalence and accepted-leaf actual Source.field replay; no GPU execution or motion inspection.",
        "source_recipe_sha256":sourcehash,"source_shape_sha256":source.shape_sha256,
        "input_hashes":{"old_density":sha(olddata),"old_header":sha(oldjson),"ranked_density":sha(newdata),"ranked_header":sha(newjson),"unchanged_diagnostics":sha(newdiag)},
        "all_frames_exactly_equal":True,"decoded_frame_count":len(frame_hashes),"frame_hashes":frame_hashes,
        "times_equal":True,"source_descriptors_equal":True,"diagnostics_byte_identical":True,
        "logical_frame_brick_modes_including_duplicate_chunks":counts,
        "admitted_runtime_product_bytes":admitted,"admitted_runtime_product_MiB":admitted/1048576.,
        "source_cap_11MiB_pass":admitted<=11*1048576,
        "evidence_diagnostics_bytes_excluded_from_runtime_product":len(newdiag),
        "source_field_evaluations":source_evaluations,"accepted_leaf_count":len(leaf_receipts),
        "quarter_reference_count":len(quarter_checks),"roi_min_zyx":lo.tolist(),"roi_max_zyx":hi.tolist(),
        "endpoint_quantization_max_tau_error":max(r["max_axial_tau_error"] for r in endpoint_errors),
        "endpoint_quantization_max_T_abs_error":max(r["max_axial_T_abs_error"] for r in endpoint_errors),
        "pre_half_maxima":maxima("pre_half"),"R16F_maxima":maxima("R16F"),
        "pre_half_failure_count":len(pre_failures),"R16F_failure_count":len(half_failures),
        "R16F_failures":half_failures,"pre_half_failures":pre_failures,
        "quarter_checks":quarter_checks,"endpoint_checks":endpoint_errors,
        "elapsed_audit_wall_s":time.perf_counter()-begin,
        "limits":"Three axial ray families at accepted-leaf quarter times. Not a continuous-time supremum or arbitrary-direction ray bound, visual motion acceptance, malformed-input security review, or runtime hardware benchmark."}
    output=BASE/"review/evaluator-ranked-audit.json";output.write_text(json.dumps(report,indent=2)+"\n")
    print(output,flush=True)
    print(json.dumps({k:report[k] for k in ["admitted_runtime_product_bytes","all_frames_exactly_equal","decoded_frame_count","source_field_evaluations","pre_half_maxima","R16F_maxima","pre_half_failure_count","R16F_failure_count","elapsed_audit_wall_s"]},indent=2),flush=True)
if __name__=="__main__":main()
