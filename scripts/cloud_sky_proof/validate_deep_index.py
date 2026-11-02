"""Standalone CPU validation of actual K48/512 endpoint-index GPU readbacks.

Usage: python3.14 validate_deep_index.py CAPTURE_DIR --receipt NAME.json
       [--baseline PATH_TO_RECEIPT.json] [--output REPORT.json]

No candidate imports, source cook, browser or GPU calls. Undefined index rows
(count2/invalid) are NEVER read. Finite nextafter checks use NumPy float32
arithmetic, not a uniform WGSL certificate. This validates unchanged segment
lookup, not performance, lateral/point-T quality or a whole cloud profile.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

import numpy as np

SCHEMA = 'thrum.deep-index-offline.v1'
COLUMNS, MAX_KNOTS, INTERVALS = 66*66, 48, 512
LAYOUTS = {'nodes': ([66,66,48,2], 1672704, 'little-endian float32'),
           'metadata': ([66,66,8], 139392, 'little-endian float32'),
           'index': ([66,66,128], 2230272, 'little-endian uint32'),
           'paths': (None, None, 'little-endian float32'),
           'field': (None, None, 'little-endian float16')}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read_receipt(path, index_required=True):
    path = Path(path).resolve()
    raw = path.read_bytes()
    receipt = json.loads(raw)
    require(receipt['schema'] == 'thrum.deep-sun-probe.v1', 'unsupported receipt schema')
    dimensions = receipt['logical_dimensions']
    require(len(dimensions)==3 and all(type(n) is int and 1<=n<=256 for n in dimensions), 'logical field dimensions')
    physical = [dimensions[0]+2, dimensions[1]+2, (dimensions[2]+3)//4+2]
    require(receipt['field_physical_dimensions']==physical, 'physical field dimensions')
    columns = receipt['selected_columns']
    require(0<len(columns)<=16, 'selected path count')
    shapes = dict(field=[physical[2],physical[1],physical[0],4], paths=[len(columns),513,4])
    payloads = {}
    names = LAYOUTS if index_required else ('nodes','metadata','field','paths')
    for name in names:
        entry = receipt['binaries'][name]
        shape, count, prefix = LAYOUTS[name]
        expected_shape = shape if shape is not None else shapes[name]
        expected_bytes = count if count is not None else int(np.prod(expected_shape))*(2 if name=='field' else 4)
        require(entry['shape']==expected_shape and entry['bytes']==expected_bytes, f'{name} descriptor shape/size')
        require(isinstance(entry['format'],str) and entry['format'].startswith(prefix), f'{name} endian/format')
        filename = (path.parent/entry['filename']).resolve()
        require(filename.is_relative_to(path.parent), 'binary path outside capture directory')
        data = filename.read_bytes()
        require(len(data)==expected_bytes and sha(data)==entry['sha256'], f'{name} actual SHA/length')
        payloads[name] = data
    words = receipt['uniform_u32']
    require(len(words)==48 and all(type(n) is int and 0<=n<=0xffffffff for n in words), 'exact Scene48u32')
    uniform = np.array(words,dtype='<u4')
    require(sha(uniform.tobytes())==receipt['identity']['uniform_sha256'], 'Scene SHA mismatch')
    require(np.array_equal(uniform[40:43], dimensions), 'Scene field-grid mismatch')
    require(receipt.get('generation_valid') is True, 'capture generation unavailable')
    return receipt, raw, payloads


def validate_index(capture_directory, receipt_name, baseline=None):
    root = Path(capture_directory).resolve()
    capture_path = (root/receipt_name).resolve()
    require(capture_path.is_relative_to(root), 'receipt outside capture directory')
    receipt, raw, payloads = read_receipt(capture_path)
    identities = {name:dict(sha256=sha(data),bytes=len(data)) for name,data in payloads.items()}
    baseline_pin = None
    if baseline is not None:
        baseline_path = Path(baseline)
        if not baseline_path.is_absolute():
            baseline_path = root/baseline_path
        baseline_has_index = 'index' in json.loads(baseline_path.read_bytes()).get('binaries',{})
        _, previous_raw, previous = read_receipt(baseline_path,index_required=baseline_has_index)
        for name in previous:
            identities[name]['baseline_byte_identical'] = payloads[name]==previous[name]
            require(identities[name]['baseline_byte_identical'], f'{name} changed from requested baseline')
        baseline_pin = dict(filename=baseline_path.name,sha256=sha(previous_raw))
    nodes = np.frombuffer(payloads['nodes'],dtype='<f4').reshape(COLUMNS,MAX_KNOTS,2)
    metadata = np.frombuffer(payloads['metadata'],dtype='<f4').reshape(COLUMNS,8)
    index = np.frombuffer(payloads['index'],dtype='<u4').reshape(COLUMNS,128)
    paths = np.frombuffer(payloads['paths'],dtype='<f4').reshape(-1,513,4)
    require(np.isfinite(metadata).all() and (metadata>=0).all(), 'nonfinite/negative metadata')
    for word in (0,1,4,5,6,7):
        require((metadata[:,word]==np.floor(metadata[:,word])).all(), 'noninteger metadata count/flag')
    require((metadata[:,0]<=MAX_KNOTS).all() and (metadata[:,1]<=1).all(), 'metadata count/valid cap')
    admitted = metadata[:,1]==1
    require((metadata[admitted,0]>=2).all() and (metadata[admitted,7]==0).all(), 'admitted metadata failure/count')
    require((metadata[:,4]<=512).all() and (metadata[:,5]<=2*metadata[:,4]).all()
            and (metadata[:,6]<=48).all(), 'metadata work cap')
    require(np.isfinite(paths).all() and (paths[:,:,3]==1).all() and (paths[:,:,1:3]>=0).all(), 'finite valid nonnegative source paths')
    require((np.diff(paths[:,:,0],axis=1)>0).all() and (np.diff(paths[:,:,1],axis=1)>=0).all(), 'ordered source path depths/tau')
    require((paths[:,0,:3]==0).all(), 'source path initial endpoint')
    grid = paths[0,:,0].copy()
    require(all(np.array_equal(p[:,0].view(np.uint32),grid.view(np.uint32)) for p in paths), 'inconsistent original endpoint grids')
    span = grid[-1]
    step = np.float32(span/np.float32(INTERVALS))
    require(np.isfinite(span) and span>0 and step>=np.finfo(np.float32).tiny, 'finite normal uniform grid required')
    reconstructed = np.float32(np.arange(513,dtype=np.float32)*step)
    require(np.array_equal(grid.view(np.uint32),reconstructed.view(np.uint32)), 'original512 grid is not bitwise uniform producing grid')
    # One grid per projection. Check selected identities and active nodes in ALL
    # admitted columns, then inspect table bytes only for defined count>2 rows.
    for selected in receipt['selected_columns']:
        x,y = selected['physical_xy']
        require(type(x) is int and type(y) is int and 0<=x<66 and 0<=y<66
                and selected['linear_index']==y*66+x, 'selected column identity')
    defined = admitted & (metadata[:,0]>2)
    matched_anchors = 0
    for column in np.flatnonzero(admitted):
        count = int(metadata[column,0])
        active = nodes[column,:count]
        require(np.isfinite(active).all() and (active[:,1]>=0).all()
                and (np.diff(active[:,0])>0).all() and (np.diff(active[:,1])>=0).all(), 'admitted active node shape/order/tau')
        require(active[0,0]==0 and active[0,1]==0 and active[-1,0]==span, 'complete source endpoint nodes')
        positions = np.searchsorted(grid,active[:,0],side='left')
        require((positions<=512).all() and np.array_equal(grid[positions].view(np.uint32),active[:,0].view(np.uint32)), 'stored anchor not an original endpoint')
        if defined[column]:
            matched_anchors += count
    requested = np.array([q['projected_depth_m'] for q in receipt['query']['rows'] if 'projected_depth_m' in q],dtype=np.float32)
    require(np.isfinite(requested).all(), 'nonfinite requested query depth')
    probes = np.unique(np.r_[grid,np.nextafter(grid,np.float32(-np.inf)),np.nextafter(grid,np.float32(np.inf)),
                             np.float32((grid[:-1]+grid[1:])*np.float32(.5)),requested])
    probes = probes[(probes>0)&(probes<span)]
    proxy = np.clip(np.floor(np.float32(np.float32(probes/span)*np.float32(INTERVALS))).astype(int),0,511)
    entries=cases=corrections=0
    maximum_distance=0
    anchor_advances=0
    for column in np.flatnonzero(defined):
        count = int(metadata[column,0])
        depths = nodes[column,:count,0]
        positions = np.searchsorted(grid,depths,side='left')
        expected = np.clip(np.searchsorted(positions,np.arange(512),side='right')-1,0,count-2)
        actual = ((index[column,:,None]>>np.array([0,8,16,24],dtype=np.uint32))&255).astype(int).reshape(512)
        require(((actual>=0)&(actual<count-1)).all(), f'index byte outside logical segment range: column{column}')
        require(np.array_equal(actual,expected), f'index differs from original interval class: column{column}')
        target = np.clip(np.searchsorted(depths,probes,side='right')-1,0,count-2)
        initial = actual[proxy]
        maximum_distance = max(maximum_distance,int(np.max(np.abs(initial-target))))
        low,high = probes<depths[initial], probes>=depths[initial+1]
        corrected = initial-low.astype(int)+high.astype(int)
        require(((corrected>=0)&(corrected<count-1)).all(), 'corrected logical segment outside nodes')
        require(((probes>=depths[corrected])&(probes<depths[corrected+1])).all()
                and np.array_equal(corrected,target), 'one-step final bracket failure')
        entries+=512;cases+=len(probes);corrections+=int(np.count_nonzero(low|high));anchor_advances+=count-2
    work = dict(rows=int(defined.sum()),interval_visits=entries,u32_word_stores=entries//4,anchor_advances=anchor_advances)
    if 'index_build' in receipt['metadata_summary']:
        require(all(receipt['metadata_summary']['index_build'][k]==v for k,v in work.items()), 'derived index work differs from receipt')
    rows = receipt['query']['rows']
    require(0<len(rows)<=128, 'bounded GPU query row count')
    indexed = bool(receipt['uniform_u32'][43]&64) and receipt['query'].get('indexed_lookup_exercised') is True
    forced = receipt['query'].get('binary_reference_forced_locally') is True
    parity=True;diagnostics_valid=True;valid_queries=0;GPU_corrections=GPU_retries=0;maximum_error=np.zeros(2)
    for row in rows:
        selected = np.asarray(row['terms'],dtype=np.float32)
        binary = np.asarray(row['binary_reference_terms'],dtype=np.float32)
        require(selected.shape==(2,) and binary.shape==(2,) and np.isfinite(selected).all() and np.isfinite(binary).all(), 'finite two-channel GPU terms')
        equal = np.array_equal(selected.view(np.uint32),binary.view(np.uint32))
        require(bool(row['selected_vs_binary_bitwise_equal'])==equal, 'reported GPU parity disagrees with decoded bits')
        valid = row['valid'] is True and row['binary_reference_valid'] is True
        if valid:
            require(((selected>=0)&(selected<=1)).all() and ((binary>=0)&(binary<=1)).all(), 'admitted GPU terms outside0..1')
            valid_queries+=1
        parity &= bool(equal and valid)
        maximum_error=np.maximum(maximum_error,np.abs(selected.astype(float)-binary.astype(float)))
        diagnostic=row['index_lookup_diagnostics']
        diagnostics_valid &= diagnostic.get('valid') is True
        require(type(diagnostic['binary_retries']) is int and type(diagnostic['adjacent_corrections']) is int
                and 0<=diagnostic['binary_retries']<=4 and 0<=diagnostic['adjacent_corrections']<=4, 'bounded GPU lookup diagnostics')
        GPU_retries+=diagnostic['binary_retries'];GPU_corrections+=diagnostic['adjacent_corrections']
    aggregate=receipt['query']['index_lookup_diagnostics']
    require(aggregate['valid_points']==valid_queries and aggregate['binary_retries']==GPU_retries
            and aggregate['adjacent_corrections']==GPU_corrections, 'GPU lookup diagnostic aggregation')
    no_errors=all(v==0 for v in receipt['query']['counters'].values())
    return dict(schema=SCHEMA,validator_sha256=sha(Path(__file__).read_bytes()),capture=capture_path.name,capture_sha256=sha(raw),
                baseline=baseline_pin,identity=receipt['identity'],binary_pins=identities,
                capture_structurally_valid=True,finite_index_gate_pass=bool(indexed and forced and parity and diagnostics_valid and no_errors),
                metadata_columns=COLUMNS,admitted_columns=int(admitted.sum()),defined_rows=int(defined.sum()),
                skipped_count2_rows=int((admitted&(metadata[:,0]==2)).sum()),skipped_invalid_rows=int((~admitted).sum()),
                decoded_defined_entries=entries,all_defined_entries_equal_original_interval_segment=True,
                matched_defined_row_anchors=matched_anchors,input_grid_bitwise_reconstructed=True,
                source_grid_span_m=float(span),source_grid_step_m=float(step),tested_nonterminal_depths=len(probes),
                defined_row_depth_cases=cases,maximum_compressed_segment_distance=maximum_distance,
                adjacent_corrections_required_CPU=corrections,final_bracket_failures=0,derived_build_work=work,
                GPU_query_count=len(rows),GPU_valid_query_count=valid_queries,indexed_selector_exercised=indexed,
                binary_reference_forced=forced,GPU_bitwise_binary_parity=parity,GPU_max_binary_absolute_error=maximum_error.tolist(),
                GPU_diagnostics_valid=bool(diagnostics_valid),GPU_lookup_diagnostics=aggregate,GPU_error_counters=receipt['query']['counters'],
                numerical_scope='finite actual f32 table/endpoints and NumPy nextafter cases; no uniform WGSL/FMA/division certificate',
                scope='unchanged compressed-segment operator only; inactive index bytes unspecified/unread; no source, lateral/point-T, performance or whole-profile pass')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture_directory',type=Path)
    parser.add_argument('--receipt',required=True)
    parser.add_argument('--baseline',type=Path)
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    try:
        result=validate_index(args.capture_directory,args.receipt,args.baseline)
        status=0 if result['finite_index_gate_pass'] else 1
    except (ValueError,KeyError,TypeError,OSError,IndexError) as error:
        result=dict(schema=SCHEMA,capture_structurally_valid=False,finite_index_gate_pass=False,error=str(error))
        status=2
    encoded=json.dumps(result,separators=(',',':'),allow_nan=False)+'\n'
    if args.output:
        args.output.write_text(encoded)
    else:
        print(encoded,end='')
    return status


if __name__=='__main__':
    sys.exit(main())
