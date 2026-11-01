"""Targeted independent endpoint R16F check, supplementing ranked audit."""
import hashlib, importlib.util, json
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[3]
BASE=ROOT/"tmp/cloud-source-proof"
def main():
    helperpath=Path(__file__).with_name("evaluator-ranked-audit.py")
    spec=importlib.util.spec_from_file_location("independent_decoder",helperpath)
    helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
    sp=ROOT/"scripts/cloud_source_proof/authored.py"
    sourcehash=hashlib.sha256(sp.read_bytes()).hexdigest()
    spec=importlib.util.spec_from_file_location("actual_source",sp)
    sm=importlib.util.module_from_spec(spec);spec.loader.exec_module(sm)
    source=sm.Source()
    m=json.loads((BASE/"authored-ranked/cloudclip.json").read_text())
    data=(BASE/"authored-ranked/density.bin").read_bytes()
    assert sourcehash==m["source_descriptor"]["source_recipe_sha256"]
    p=np.argwhere(source.base>0);roi=tuple(slice(int(a),int(b)+1) for a,b in zip(p.min(0),p.max(0)))
    results=[]
    for t,record in zip(m["source_times_s"],m["density_frames"]):
        codes,_=helper.decode(data,record,True)
        val=(codes[roi].astype(np.float32)*np.float32(.125/255.)).astype(np.float16).astype(np.float64)
        ref=source.field(t)[roi].astype(np.float64)
        results.append({"source_time_s":t,**helper.optical_error(val,ref)})
    report={"recipe_sha256":sourcehash,"scope":"All162 exact runtime knot fields after R16F storage; independent ranked decoder and actual Source.field.",
        "endpoint_count":len(results),"checks":results,
        "max_axial_tau_error":max(r["max_axial_tau_error"] for r in results),
        "max_axial_T_abs_error":max(r["max_axial_T_abs_error"] for r in results),
        "failure_count":sum(r["max_axial_tau_error"]>.1 or r["max_axial_T_abs_error"]>.02 for r in results)}
    assert hashlib.sha256(sp.read_bytes()).hexdigest()==sourcehash
    output=BASE/"review/evaluator-ranked-endpoint-half.json";output.write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps({k:report[k] for k in ["endpoint_count","max_axial_tau_error","max_axial_T_abs_error","failure_count"]},indent=2))
if __name__=="__main__":main()
