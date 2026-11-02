"""CPU-only fixed-depth T1/Tquarter lattice comparison against actual PL tau.

D active planes INCLUDE both support endpoints; two duplicated guard planes are
additional. Query linearly interpolates TRANSMITTANCE in depth and XY. This is
not exp(linearly interpolated tau), not point-visibility admission, and not a GPU
speed/backend certificate. Source quadrature is outside the comparison.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np

EXPONENTS = np.array([1., .25])
PLANES = (64, 128, 256)
DEPTH_ALLOCATION = .005
STORAGE_ALLOCATION = .002
CPU_NUMERIC_GUARD = 2e-12


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def validate_original(original):
    a = np.asarray(original, dtype=np.float64)
    require(a.ndim == 2 and a.shape[1] == 2 and len(a) >= 2 and np.isfinite(a).all(), 'finite depth/tau pairs')
    require((np.diff(a[:, 0]) > 0).all() and (np.diff(a[:, 1]) >= 0).all() and (a[:, 1] >= 0).all(), 'ordered nonnegative cumulative tau')
    return a


def lattice(original, planes):
    original = validate_original(original)
    require(planes in PLANES, 'bounded comparison is64/128/256 active planes')
    # Mathematical uniform centres. A GPU's f32 coordinate/exp differences are
    # separate, unqualified backend error; this oracle does not hide them here.
    depth = np.linspace(original[0, 0], original[-1, 0], planes)
    tau = np.interp(depth, original[:, 0], original[:, 1])
    terms = np.exp(-tau[:, None]*EXPONENTS)
    stored = terms.astype('<f2').astype(np.float64)
    return depth, terms, stored


def transformed_error(original, depth, values):
    """Analytic sup |exp(-a*tau_PL)-linear_T_lattice|, both active branches.

    Merge input knots/lattice planes. On each piece old tau=t+d*u and
    new T=c+e*u. Critical derivative: exp(-a*(t+d*u))=-e/(a*d).
    With d=0 or e=0 extrema are endpoints; otherwise test the one interior root.
    Unlike tau-knot compression, lattice T is linear, not exponential-of-linear.
    """
    original = validate_original(original)
    depth, values = np.asarray(depth, float), np.asarray(values, float)
    require(depth.ndim == 1 and values.shape == (len(depth), 2) and np.isfinite(values).all()
            and ((values >= 0) & (values <= 1)).all() and (np.diff(depth) > 0).all(), 'ordered finite T lattice')
    require(depth[0] == original[0, 0] and depth[-1] == original[-1, 0], 'lattice complete endpoint domain')
    require((np.diff(values, axis=0) <= 0).all(), 'monotone transmittance')
    split = np.unique(np.r_[original[:, 0], depth])
    tau = np.interp(split, original[:, 0], original[:, 1])
    maxima, witnesses, signed_errors = [], [], []
    locations = 0
    for branch, a in enumerate(EXPONENTS):
        linear = np.interp(split, depth, values[:, branch])
        difference = linear-np.exp(-a*tau)
        index = int(np.argmax(np.abs(difference)))
        maximum, witness, signed = abs(float(difference[index])), float(split[index]), float(difference[index])
        locations += len(split)
        for i in range(len(split)-1):
            d, e = tau[i+1]-tau[i], linear[i+1]-linear[i]
            if d == 0 or e == 0:
                continue
            # Positive original slopes and nonpositive T slopes are validated.
            log_target = math.log(-e)-math.log(a)-math.log(d)
            u = (-log_target/a-tau[i])/d
            if 0 < u < 1:
                target = math.exp(-a*(tau[i]+d*u))
                difference = linear[i]+e*u-target
                locations += 1
                if abs(difference) > maximum:
                    maximum, signed = abs(difference), difference
                    witness = float(split[i]+u*(split[i+1]-split[i]))
        maxima.append(maximum)
        witnesses.append(witness)
        signed_errors.append(signed)
    return dict(error=maxima, witness_depth_m=witnesses, signed_lattice_minus_reference=signed_errors,
                analytic_comparison_locations=locations)


def dense_check(original, depth, values, subdivisions=32):
    original = validate_original(original)
    x = np.concatenate([np.linspace(a, b, subdivisions+1) for a, b in zip(original[:-1, 0], original[1:, 0])])
    tau = np.interp(x, original[:, 0], original[:, 1])
    linear = np.column_stack([np.interp(x, depth, values[:, j]) for j in range(2)])
    return np.max(np.abs(linear-np.exp(-tau[:, None]*EXPONENTS)), axis=0).tolist()


def analyze_path(path, planes):
    original = validate_original(np.asarray(path)[:, :2])
    depth, ideal, stored = lattice(original, planes)
    approximation = transformed_error(original, depth, ideal)
    storage = np.max(np.abs(stored-ideal), axis=0).tolist()
    combined = transformed_error(original, depth, stored)
    dense = dense_check(original, depth, stored)
    require(all(x <= y+CPU_NUMERIC_GUARD for x, y in zip(dense, combined['error'])), 'dense query exceeds analytic extrema estimate')
    return dict(planes=planes, depth_approximation=approximation, half_storage_error=storage,
                combined=combined, dense_combined_error=dense,
                depth_admitted=max(approximation['error'])+CPU_NUMERIC_GUARD <= DEPTH_ALLOCATION,
                storage_admitted=max(storage) <= STORAGE_ALLOCATION)


def compare_capture(receipt_path):
    receipt_path = Path(receipt_path)
    receipt_bytes = receipt_path.read_bytes()
    receipt = json.loads(receipt_bytes)
    require(receipt['schema'] == 'thrum.deep-sun-probe.v1', 'actual probe schema')
    entry = receipt['binaries']['paths']
    payload = (receipt_path.parent/entry['filename']).read_bytes()
    require(sha(payload) == entry['sha256'] and len(payload) == entry['bytes'], 'actual raw-path SHA/size')
    columns = receipt['selected_columns']
    require(0 < len(columns) <= 16 and len(payload) == len(columns)*513*16, 'actual raw-path shape')
    paths = np.frombuffer(payload, dtype='<f4').reshape(len(columns), 513, 4)
    require(np.isfinite(paths).all() and (paths[:, :, 3] == 1).all(), 'finite valid actual source paths')
    records, summaries = [], []
    for count in PLANES:
        rows = []
        for column, path in zip(columns, paths):
            row = analyze_path(path, count)
            row.update(physical_xy=column['physical_xy'], selection_kind=column['selection_kind'])
            rows.append(row)
        def maximum(key):
            return np.max([r[key] for r in rows], axis=0).tolist()
        depth_error = np.max([r['depth_approximation']['error'] for r in rows], axis=0).tolist()
        combined_error = np.max([r['combined']['error'] for r in rows], axis=0).tolist()
        summaries.append(dict(planes=count, all_selected_depth_admitted=all(r['depth_admitted'] for r in rows),
                              rejected_columns=sum(not r['depth_admitted'] for r in rows), depth_error_max=depth_error,
                              half_storage_error_max=maximum('half_storage_error'), combined_error_max=combined_error))
        records.extend(rows)
    return dict(receipt_filename=receipt_path.name, receipt_sha256=sha(receipt_bytes), raw_paths_sha256=sha(payload),
                identity=receipt['identity'], source_time_s=receipt['source_time_s'], profile=receipt['profile'],
                projection=receipt['projection'], selected_columns=len(columns), summaries=summaries, rows=records)


def comparison(receipts):
    return dict(schema='thrum.dense-light-reference.v1', tool_sha256=sha(Path(__file__).read_bytes()),
                contract=dict(active_planes_include_both_endpoints=True, physical_depth_planes='D+2 duplicate endpoint guards',
                              exponents=EXPONENTS.tolist(), quadrature_allocation=.005, depth_allocation=.005, storage_allocation=.002,
                              numeric_guard=CPU_NUMERIC_GUARD, uniform_GPU_backend_certificate=False,
                              operator='linear interpolation of T per depth, then separate nonnegative XY weighting; same filtered footprint'),
                captures=[compare_capture(path) for path in receipts],
                costs=[dict(active_planes=d, physical_planes=d+2, texels=66*66*(d+2), bytes_RGBA16F=66*66*(d+2)*8,
                            output_exponentials=66*66*d*2, source_coefficient_requests=66*66*512,
                            K48_curve_metadata_bytes=66*66*(48*8+32)) for d in PLANES],
                scope='finite actual raw PL-tau depth-lattice comparison; no source, point-visibility, filtered-coverage, appearance or GPU-speed pass')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('receipts', type=Path, nargs='+')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    text = json.dumps(comparison(args.receipts), separators=(',', ':'), allow_nan=False)+'\n'
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end='')


if __name__ == '__main__':
    main()
