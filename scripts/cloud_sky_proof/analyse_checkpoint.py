"""Rebuild the finite checkpoint summary from retained actual-canvas receipts."""
from pathlib import Path
import argparse
import hashlib
import json
import numpy as np
from PIL import Image

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def analyse(root):
    cost_path=root/'cost-330-frames.json'
    cost=json.loads(cost_path.read_text())
    rows=[row for row in cost['samples'] if not row['warmup']]
    def percentiles(values):
        a=np.asarray(values,dtype=np.float64)
        if not len(a) or not np.isfinite(a).all():
            raise ValueError('Missing/nonfinite measurements')
        return dict(zip(['p50','p95','p99','max'],[*np.percentile(a,[50,95,99],method='linear'),float(a.max())]))
    timing={key:percentiles([row[key] for row in rows]) for key in ['gpu_bracket_ms','queue_complete_ms','cpu_encode_ms']}
    timing['callback_complete_cadence_ms']=percentiles(np.diff([row['wall_complete_s'] for row in rows])*1000)
    motions=[]
    for path in sorted((root/'motion').glob('*.json')):
        d=json.loads(path.read_text());s=d['samples']
        motions.append({'receipt':path.name,'sha256':digest(path),'frames':len(s),'rate':d['rate'],'source_start_s':s[0]['source_time_s'],'source_end_s':s[-1]['source_time_s'],'wall_complete_s':s[-1]['wall_complete_s'],'invalid_view_paths':sum(row['invalid_view_paths'] for row in s),'native_snapshots':len(d['snapshots'])})
    fit=[]
    for path in sorted(root.glob('fit-sun*-probe.json')):
        d=json.loads(path.read_text())
        fit.append({'receipt':path.name,'sha256':digest(path),'implementation_checks_pass':d['implementation_checks_pass'],'LUT_error_max':d['LUT']['max_absolute_error'],'centre_A_storage_error_max':d['centre_bank_storage']['max_absolute_error'],'canonical_column_T_error_max':d['body_quality']['T_error']['max_absolute_error'],'A_vs_canonical_column_error_max':d['body_quality']['A_vs_canonical_T_error']['max_absolute_error'],'sampled_Amap_nonlinearity_error_max':d['A_map_nonlinearity']['max_absolute_error']})
    passive=np.asarray(Image.open(root/'images/passive-coupled.png').convert('RGB')).astype(np.int16)
    fitted=np.asarray(Image.open(root/'images/fit-coupled-final.png').convert('RGB')).astype(np.int16)
    dark=np.asarray(Image.open(root/'images/all-off.png').convert('RGB'))
    default=np.asarray(Image.open(root/'images/fit-coupled.png').convert('RGB')).astype(np.int16)
    hidden=np.asarray(Image.open(root/'images/main-hidden.png').convert('RGB')).astype(np.int16)
    field_bytes=rows[0]['source_field_bytes'];fit_bytes=rows[0]['scatter_fit_bytes']
    logical_gpu={'source_field_with_guards_mips':field_bytes,'encoded_source':6491440,'prefix':66**3*8,'ambient':8**3*8,'shared_fit':fit_bytes['sharedGPU'],'body_and_bank':fit_bytes['products']['total'],'scene_errors_queries_marker_minimum':192+16+256+16+4,'field_and_prepared_uniforms':3*64+2*16+64,'one_visible_1280x720_BGRA_texture':1280*720*4}
    return {'schema':'thrum.cloud-correction-summary.v1','scope':'One retained Q1 source,128 field,native1280x720,local low-altitude coupling fixture. Not full SKY-G,full-sky or engine/device acceptance. Timing scope is actual first compute through actual rendered-texture consuming end marker; queue/cadence include measurement readbacks.','cost_input_sha256':digest(cost_path),'cost_build':cost['identity']['build_id'],'numpy':np.__version__,'warmup_frames':30,'measured_frames':len(rows),'fresh_source_frames':sum(row['source_updated'] for row in rows),'fresh_light_frames':sum(row['light_updated'] for row in rows),'invalid_view_paths':sum(row['invalid_view_paths'] for row in rows),'timing_ms':timing,'cloud_GPU_5ms_gate':'FAIL','complete_engine_browser_GPU_16ms_gate':'UNRUN; standalone fixture p99 already exceeds16ms','CPU_0_5ms_gate':'UNQUALIFIED: encode only, not complete CPU work','logical_GPU_allocations_bytes':logical_gpu,'logical_GPU_sum_bytes':sum(logical_gpu.values()),'transient_error_readback_bytes':16,'memory_scope':'Known logical subtotal of declared resources plus one canvas texture; driver/browser/swapchain multiplicity/residency/process peak unmeasured. Replacement has new+old overlap. CPU encoded source6491440B,metadata73104B,fit recovery292941B before parsed-object/compiler overhead.','motions':motions,'fit_probes':fit,'image_checks':{'all_off_max_RGB8':int(dark.max()),'main_hidden_shadow_patch_xyxy':[750,390,1100,440],'shadow_patch_max_RGB8_difference':int(np.abs(default[390:440,750:1100]-hidden[390:440,750:1100]).max()),'passive_vs_fit_receiver_patch_max_RGB8_difference':int(np.abs(passive[390:440,750:1100]-fitted[390:440,750:1100]).max()),'scope':'Selected receiver pixels only, not a global optical/radiance certificate'}}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('evidence',type=Path);parser.add_argument('--output',type=Path)
    args=parser.parse_args();data=json.dumps(analyse(args.evidence),indent=2)+'\n'
    if args.output:args.output.write_text(data)
    else:print(data,end='')
