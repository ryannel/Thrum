"""Bounded adaptive direct-volume preparation from the continuous authored source.

Every leaf tests actual authoring-function quarter/mid/three-quarter fields.
No reference field is reconstructed from cooked frames. No carrier/history.
"""
from __future__ import annotations

import argparse
from collections import OrderedDict
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import time

import numpy as np

ROOT = Path(__file__).resolve().parent
TICK_SECONDS = .1171875
END = 2048
INITIAL = 64
MINIMUM = 4
CODE_MAX = .125


def module(name):
    path = ROOT/(name+'.py')
    spec = importlib.util.spec_from_file_location('authored_compile_'+name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


codec = module('cook')
direct = module('direct')


class BoundReached(Exception):
    pass


class InvalidSource(Exception):
    pass


class Compiler:
    def __init__(self, source, wall_seconds=180., max_evaluations=4096):
        self.source = source
        self.descriptor = source.descriptor()
        self.side = int(self.descriptor['density_side'])
        self.extent = float(self.descriptor['extent_m'])
        self.begin = time.perf_counter()
        self.wall_seconds = wall_seconds
        self.max_evaluations = max_evaluations
        self.fields = OrderedDict()
        self.peak_fields = 0
        self.evaluations = 0
        self.sampled = {}
        self.quantization = {}
        self.knots = {}
        self.chunks = {}
        self.payload = bytearray()
        self.nodes = []
        self.leaves = []
        self.unresolved = []
        self.failures = []

    def field(self, tick):
        if tick in self.fields:
            self.fields.move_to_end(tick)
            return self.fields[tick]
        if time.perf_counter()-self.begin >= self.wall_seconds or self.evaluations >= self.max_evaluations:
            raise BoundReached('explicit wall/evaluation limit reached; source qualification incomplete')
        # Evict before evaluating: at most8 source field arrays are retained.
        if len(self.fields) >= 8:
            self.fields.popitem(last=False)
        age = tick*TICK_SECONDS
        field = np.asarray(self.source.field(age))
        self.evaluations += 1
        if field.dtype != np.float32 or field.shape != (self.side,)*3:
            raise InvalidSource(f'{age}s: source must return native cubicfloat32sigma')
        if not np.isfinite(field).all() or float(field.min()) < 0 or float(field.max()) > CODE_MAX:
            raise InvalidSource(f'{age}s: source outside fixedfinite[0,{CODE_MAX}]sigma range; no clamp')
        k = min(8, self.side//4)
        collar = max(float(a.max()) for a in
            (field[:k],field[-k:],field[:,:k],field[:,-k:],field[:,:,:k],field[:,:,-k:]))
        if collar != 0:
            self.failures.append(f'{age}s: nonzero eight-voxel validitycollar')
        if tick in (0, END) and float(field.max()) != 0:
            self.failures.append(f'{age}s: endpoint is notexactzero')
        self.sampled[tick] = {'source_time_s':age, 'max_sigma_per_m':float(field.max()),
                             'boundary_collar_max_sigma_per_m':collar,
                             'float32_reference_sha256':hashlib.sha256(field.tobytes()).hexdigest()}
        field.setflags(write=False)
        self.fields[tick] = field
        self.peak_fields = max(self.peak_fields,len(self.fields))
        if self.evaluations % 50 == 0:
            print(f'AUTHORING_EVALUATIONS {self.evaluations} elapsed_s={time.perf_counter()-self.begin:.2f}',flush=True)
        return field

    def encode(self, tick):
        if tick not in self.knots:
            actual = self.field(tick)
            codes = np.rint(actual/CODE_MAX*255).astype(np.uint8)
            payload, record = codec.pack_density(codes)
            code_hash = hashlib.sha256(codes.tobytes()).hexdigest()
            if code_hash not in self.chunks:
                record.update(offset=len(self.payload),sha256=hashlib.sha256(payload).hexdigest(),
                              quantized_frame_sha256=code_hash,chunk_id=len(self.chunks))
                self.chunks[code_hash] = record
                self.payload.extend(payload)
            else:
                old=self.chunks[code_hash]
                if bytes(self.payload[old['offset']:old['offset']+old['bytes']]) != payload:
                    raise InvalidSource('exact-frame dedup hash collision')
            self.knots[tick] = self.chunks[code_hash]
            decoded = codes.astype(np.float32)*np.float32(CODE_MAX/255)
            error = codec.optical_error(decoded,actual,self.extent/self.side)
            self.quantization[tick] = {'source_time_s':tick*TICK_SECONDS,**error}
            if error['max_axial_tau_error']>.1 or error['max_axial_transmittance_abs_error']>.02:
                self.failures.append(f'{tick*TICK_SECONDS}s: endpointquantization opticalgate failed')
            return decoded
        record = self.knots[tick]
        raw = self.payload[record['offset']:record['offset']+record['bytes']]
        return codec.unpack_density(raw,self.side,record).astype(np.float32)*np.float32(CODE_MAX/255)

    def check(self, first, last):
        a,b = self.encode(first),self.encode(last)
        checks=[]
        for numerator in (1,2,3):
            tick=first+(last-first)*numerator//4
            reference=self.field(tick)
            fraction=numerator/4
            candidate=(1-fraction)*a+fraction*b
            error=codec.optical_error(candidate,reference,self.extent/self.side)
            checks.append({'source_time_s':tick*TICK_SECONDS,**error})
            del reference,candidate
        record={'start_s':first*TICK_SECONDS,'end_s':last*TICK_SECONDS,
                'duration_s':(last-first)*TICK_SECONDS,'checks':checks,
                'max_axial_tau_error':max(c['max_axial_tau_error'] for c in checks),
                'max_axial_transmittance_abs_error':max(c['max_axial_transmittance_abs_error'] for c in checks)}
        record['passed']=record['max_axial_tau_error']<=.1 and record['max_axial_transmittance_abs_error']<=.02
        self.nodes.append(record)
        return record

    def refine(self, first, last):
        record=self.check(first,last)
        if record['passed']:
            self.leaves.append(record)
        elif last-first==MINIMUM:
            self.unresolved.append(dict(record,reason='minimum0.46875s reached; opticalgate stillfailed'))
        else:
            middle=(first+last)//2
            self.refine(first,middle)
            self.refine(middle,last)

    def run(self):
        try:
            # Fixed initial endpoints retain declared fulltime coverage if a later
            # adaptive interval reaches the explicit compute limit.
            for tick in range(0,END+1,INITIAL):
                self.encode(tick)
            for first in range(0,END,INITIAL):
                self.refine(first,first+INITIAL)
        except (BoundReached,InvalidSource) as error:
            self.failures.append(str(error))
        return self


def maximum(records,name):
    return max((r[name] for r in records),default=0.)


def write_product(compiler,out,pins):
    out.mkdir(parents=True,exist_ok=False)
    ordered=sorted(compiler.knots)
    (out/'density.bin').write_bytes(compiler.payload)
    unresolved=compiler.unresolved
    covered=sum(r['duration_s'] for r in compiler.leaves)+sum(r['duration_s'] for r in unresolved)
    failures=list(dict.fromkeys(compiler.failures))
    if unresolved:
        failures.append(f'{len(unresolved)}minimum-step intervals failedopticalgates')
    if covered!=240.:
        failures.append('adaptive leaf coverage is incomplete; no fullclip qualification')
    diagnostics={'source_descriptor':compiler.descriptor,'pins':pins,
        'gates':{'max_axial_tau_error':.1,'max_axial_transmittance_abs_error':.02,
                 'minimum_interval_s':.46875,'initial_interval_s':7.5},
        'accepted_leaf_quarter_checks':compiler.leaves,'unresolved_minimum_leaves':unresolved,
        'all_tested_nodes_including_rejected_parents':compiler.nodes,
        'actual_reference_samples':[compiler.sampled[t] for t in sorted(compiler.sampled)],
        'endpoint_quantization':[compiler.quantization[t] for t in sorted(compiler.quantization)],
        'source_field_evaluations':compiler.evaluations,'distinct_reference_times':len(compiler.sampled),
        'source_field_cache_capacity':8,'peak_retained_source_fields':compiler.peak_fields,
        'peak_retained_source_field_bytes':compiler.peak_fields*compiler.side**3*4,
        'explicit_wall_limit_s':compiler.wall_seconds,'explicit_evaluation_limit':compiler.max_evaluations,
        'compile_wall_s':time.perf_counter()-compiler.begin,'failures':failures,
        'limits':'Actual continuous authoring samples at declaredquarters, not a proof of supremum error at every unsampled real time or a full renderer/device budget.'}
    diagnostics_path=out/'diagnostics.json'
    diagnostics_path.write_text(json.dumps(diagnostics,indent=2)+'\n')
    metadata={'schema':direct.SCHEMA,'reader_kind':'direct','density_side':compiler.side,
        'flow_side':0,'flow_intervals':[],'extent_m':compiler.extent,'bounds_min_xyz_m':[0,0,0],
        'axes':'right-handedY-upXYZ; array[Z,Y,X]; XfastestthenYthenZ',
        'voxel_centres':'bounds_min+(ijk+.5)*extent/side; fixed across alltimes',
        'runtime_sigma_max_per_m':CODE_MAX,'source_times_s':[t*TICK_SECONDS for t in ordered],
        'source_frame_step_s':None,
        'density_frames':[dict(compiler.knots[t],source_frame=i+1) for i,t in enumerate(ordered)],
        'unique_density_chunks':len(compiler.chunks),
        'density_encoding':'u16LEbrickindex empty65535;littlebitorderoccupancy;512R8voxels/activebrick',
        'runtime_evaluation':'directlerp at adaptive genuine authoringtimeknots; noflow/nohistory',
        'history':'unavailable in this proof; local creation/erosion not certified for reuse',
        'time_policy':'finite0..240s; endpointzero hold; no loop',
        'source_descriptor':compiler.descriptor,'rebuild_pins':pins,
        'products':{'density.bin':{'bytes':len(compiler.payload),'sha256':codec.sha256(out/'density.bin')}},
        'evidence_products':{'diagnostics.json':{'bytes':diagnostics_path.stat().st_size,'sha256':codec.sha256(diagnostics_path)}},
        'metadata_reserved_bytes':131072,'source_cap_bytes':11*1024**2,
        'quality_gates':{'max_axial_tau_error':.1,'max_axial_transmittance_abs_error':.02,
                         'minimum_interval_s':.46875},
        'accepted_leaf_count':len(compiler.leaves),'unresolved_leaf_count':len(unresolved),
        'accepted_leaf_quarter_max_tau_error':maximum(compiler.leaves,'max_axial_tau_error'),
        'accepted_leaf_quarter_max_T_abs_error':maximum(compiler.leaves,'max_axial_transmittance_abs_error'),
        'endpoint_quantization_max_tau_error':maximum(compiler.quantization.values(),'max_axial_tau_error'),
        'endpoint_quantization_max_T_abs_error':maximum(compiler.quantization.values(),'max_axial_transmittance_abs_error'),
        'source_field_evaluations':compiler.evaluations,'distinct_reference_times':len(compiler.sampled),
        'peak_retained_source_fields':compiler.peak_fields,'compile_wall_s':time.perf_counter()-compiler.begin,
        'bounds_validation':'All actual sampled sourcefields checkedfinite/nonnegative/<=.125 and collar/endpoints; no valueclamp. Continuous globalbounds remain owned by pinned authoringformula, not inferred from sparsechecks.',
        'qualification':'rejected' if failures else 'declaredquarter/quantization/boundary tests passed; visual/unsampledtime/engine/device budgets unqualified',
        'rejection_reasons':failures}
    codec.write_manifest(out/'cloudclip.json',metadata)
    for condition,message in [(metadata['metadata_actual_bytes']>131072,'runtimeheader exceeds128KiB'),
                              (metadata['admitted_bytes_actual']>11*1024**2,'runtimeheader+source exceeds11MiB')]:
        if condition:
            metadata['rejection_reasons'].append(message)
            metadata['qualification']='rejected'
    codec.write_manifest(out/'cloudclip.json',metadata)
    # Verify every encoded knot with the separately authored direct-reader class.
    product=direct.read_product(out)
    for index,tick in enumerate(ordered):
        reference=compiler.encode(tick)
        if not np.array_equal(reference,product['frames'][index]):
            raise ValueError('direct.Frames roundtrip mismatch')
    return metadata


def self_test():
    class Fixture:
        def descriptor(self):
            return {'density_side':16,'extent_m':64.,'duration_s':240.,'history':'unavailable'}
        def field(self,s):
            a=np.zeros((16,)*3,np.float32)
            if 0<s<15:
                a[4:12,4:12,4:12]=.125*np.sin(np.pi*s/15)**2
            return a
    compiler=Compiler(Fixture(),10,4096).run()
    assert compiler.evaluations>33 and compiler.peak_fields<=8
    assert any(r['duration_s']<7.5 for r in compiler.leaves)
    assert not compiler.unresolved and not compiler.failures
    with tempfile.TemporaryDirectory() as directory:
        out=Path(directory)/'product'
        m=write_product(compiler,out,{'role':'smallanalyticsemanticfixtureonly'})
        assert m['qualification']!='rejected' and m['admitted_bytes_actual']<=11*1024**2
        assert sum(r['duration_s'] for r in compiler.leaves)==240
    class Discontinuous(Fixture):
        def field(self,s):
            a=np.zeros((16,)*3,np.float32)
            if 0<s<240:a[4:12,4:12,4:12]=.125
            return a
    bad=Compiler(Discontinuous(),10,4096).run()
    assert bad.unresolved and all(r['duration_s']==.46875 for r in bad.unresolved)
    print('AUTHORED_COMPILER_SELF_TEST_PASS: realreferenceadaptivebisection,minstepfailure,cache8,dedupreader,byteaccounting',flush=True)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',type=Path)
    parser.add_argument('--shape',type=Path)
    parser.add_argument('--wall-seconds',type=float,default=180.)
    parser.add_argument('--max-evaluations',type=int,default=4096)
    parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args()
    if args.self_test:self_test();return
    if args.out is None:parser.error('--out required')
    if not 0<args.wall_seconds<=180 or not 0<args.max_evaluations<=4096:
        parser.error('explicit bounds require wall<=180s and evaluations<=4096')
    if args.out.exists():raise ValueError('output must be a new directory')
    authored=module('authored')
    source=authored.Source(args.shape or authored.DEFAULT_SHAPE)
    descriptor=source.descriptor()
    if descriptor['density_side']!=128 or descriptor['extent_m']!=512 or descriptor['duration_s']!=240:
        raise ValueError('unexpected authored source domain/profile')
    paths={'compiler':Path(__file__),'source_recipe':ROOT/'authored.py','codec':ROOT/'cook.py',
           'reader':ROOT/'direct.py','shape':source.path/'shape.r8','shape_metadata':source.path/'shape.json',
           'license':source.path/'LICENSE.md'}
    pins={name:{'path':str(path),'sha256':codec.sha256(path)} for name,path in paths.items()}
    compiler=Compiler(source,args.wall_seconds,args.max_evaluations).run()
    for name,path in paths.items():
        if codec.sha256(path)!=pins[name]['sha256']:
            compiler.failures.append(f'{name} changed during compilation; mixed revision rejected')
    m=write_product(compiler,args.out,pins)
    print('AUTHORED_COMPILE_COMPLETE',json.dumps({key:m[key] for key in
        ['qualification','rejection_reasons','accepted_leaf_count','unresolved_leaf_count',
         'source_field_evaluations','metadata_actual_bytes','admitted_bytes_actual',
         'accepted_leaf_quarter_max_tau_error','accepted_leaf_quarter_max_T_abs_error','compile_wall_s']}),flush=True)


if __name__=='__main__':main()
