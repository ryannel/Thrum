"""All-time support receipt for the pinned prepared Q1 clip; CPU only.

Lossless mixed-codec frames are decoded with their pinned existing codec and
verified against both encoded-chunk and decoded R8 hashes. New work here is the
complete-frame positivity union and direct source -> sampled-field hat support.
The result bounds support only; it does not alter coefficients/calibration or
qualify optical/lateral quality. No GPU or candidate consumer imports.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
SELECTED = ROOT/'docs/evidence/cloud-source-2026-11-01/selected-clip'
PINS = dict(manifest='2fca53ff92a9787a0bb54bd9db0c470f5cfe2a5d755534c134eec38568acb739',
            density='999cd89480371d02dca2f7553b938e5b93e23f0b31ca9b306ae60beb62e325bd',
            decoder='c988983d157195f2d1c582fbd37469697b6c51c4f8c85498a645a7df4822c1a7')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def mask_bounds(mask):
    coords = np.array(np.nonzero(mask))
    if not coords.shape[1]:
        return None
    return dict(min_xyz=coords.min(axis=1)[::-1].tolist(), max_xyz=coords.max(axis=1)[::-1].tolist(), positive_samples=int(coords.shape[1]))


def field_support_mask(original, output_side):
    """Exact positivity at field centres for fixed source-side or twice-side grids.

    Source hats have centre 4i+2, width support ±4. For twice-side field centres
    2j+1, a source index i contributes iff j in {2i-1,2i,2i+1,2i+2}.
    Tensor-product nonnegative weights mean ANY positive contributor suffices.
    The function works in grid index space; the support rule is scale invariant.
    """
    side = original.shape[0]
    require(original.shape == (side,)*3 and original.dtype == bool, 'cubic boolean source support')
    if output_side == side:
        return original.copy()
    require(output_side == 2*side, 'only same-grid or twice-grid field admission')
    indices = np.array(np.nonzero(original)).T*2
    result = np.zeros((output_side,)*3, bool)
    for z in (-1, 0, 1, 2):
        for y in (-1, 0, 1, 2):
            for x in (-1, 0, 1, 2):
                target = indices+np.array([z, y, x])
                target = target[((target >= 0) & (target < output_side)).all(axis=1)]
                result[target[:, 0], target[:, 1], target[:, 2]] = True
    return result


def reconstructed_bounds(mask, extent_m=512.):
    """Closure of positive field sample hats, not merely their voxel cell hull."""
    bounds = mask_bounds(mask)
    require(bounds is not None, 'empty support has no finite occupied bounds')
    pitch = extent_m/mask.shape[0]
    low = (np.array(bounds['min_xyz'])+.5)*pitch-pitch
    high = (np.array(bounds['max_xyz'])+.5)*pitch+pitch
    return dict(**bounds, field_side=mask.shape[0], pitch_m=pitch,
                min_m=low.tolist(), max_m=high.tolist(), extent_m=(high-low).tolist(),
                centre_m=((low+high)/2).tolist())


def support_receipt():
    manifest_bytes = (SELECTED/'cloudclip.json').read_bytes()
    density = (SELECTED/'density.bin').read_bytes()
    decoder_path = ROOT/'scripts/cloud_source_proof/ranked.py'
    decoder_bytes = decoder_path.read_bytes()
    observed = dict(manifest=sha(manifest_bytes), density=sha(density), decoder=sha(decoder_bytes))
    require(observed == PINS, 'pinned selected source/manifest/decoder mismatch')
    metadata = json.loads(manifest_bytes)
    require(metadata['reader_kind'] == 'direct' and metadata['density_side'] == 128 and metadata['extent_m'] == 512
            and metadata['bounds_min_xyz_m'] == [0, 0, 0] and metadata['runtime_sigma_max_per_m'] == .125
            and metadata['density_encoding_version'] == 'thrum.cloud-source-proof.mixed-bricks.v1', 'unsupported prepared source contract')
    spec = importlib.util.spec_from_file_location('pinned_support_decoder', decoder_path)
    decoder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(decoder)
    union = np.zeros((128,)*3, bool)
    frames, chunks = metadata['density_frames'], {}
    times = metadata['source_times_s']
    require(len(times) == len(frames) and all(a < b for a, b in zip(times, times[1:])), 'source time knots')
    for index, record in enumerate(frames):
        key = record['chunk_id']
        signature = (record['offset'], record['bytes'], record['sha256'], record['quantized_frame_sha256'])
        if key in chunks:
            require(chunks[key]['signature'] == signature, 'inconsistent repeated chunk identity')
            chunks[key]['source_times_s'].append(times[index])
            continue
        block = density[record['offset']:record['offset']+record['bytes']]
        require(sha(block) == record['sha256'], 'encoded frame hash')
        codes = decoder.unpack_frame(block, 128, record)
        require(sha(codes.tobytes()) == record['quantized_frame_sha256'], 'decoded R8 frame hash')
        active = codes != 0
        union |= active
        chunks[key] = dict(chunk_id=key, signature=signature, source_times_s=[times[index]],
                           positive_voxels=int(active.sum()), bounds=mask_bounds(active))
    require(len(chunks) == metadata['unique_density_chunks'], 'unique chunk count')
    original = mask_bounds(union)
    fields = [reconstructed_bounds(field_support_mask(union, side)) for side in (128, 256)]
    # Each unique frame contributes only inside the all-time nonzero voxel hull.
    outside = 0
    lower, upper = np.array(original['min_xyz']), np.array(original['max_xyz'])
    for chunk in chunks.values():
        if chunk['bounds']:
            outside += int((np.array(chunk['bounds']['min_xyz']) < lower).any() or (np.array(chunk['bounds']['max_xyz']) > upper).any())
    require(outside == 0, 'unique source frame outside proposed union hull')
    for field in fields:
        # Independently check the expected exact sample-index enlargement.
        factor = field['field_side']//128
        low = lower if factor == 1 else 2*lower-1
        high = upper if factor == 1 else 2*upper+2
        require(field['min_xyz'] == low.tolist() and field['max_xyz'] == high.tolist(), 'actual field-centre support disagreement')
    for c in chunks.values():
        del c['signature']
    return dict(schema='thrum.prepared-support.v1', tool_sha256=sha(Path(__file__).read_bytes()), input_hashes=observed,
                extent_m=512, sigma_max_per_m=.125, calibration_only=True,
                source_knots=len(frames), unique_chunks=len(chunks),
                active_frame_instances=sum(chunks[r['chunk_id']]['positive_voxels'] > 0 for r in frames),
                active_unique_chunks=sum(c['positive_voxels'] > 0 for c in chunks.values()),
                inactive_times_s=[t for t, r in zip(times, frames) if chunks[r['chunk_id']]['positive_voxels'] == 0],
                original_voxel_union=original, reconstructed_fields=fields, outside_proposed_source_hull=outside,
                chunks=list(chunks.values()),
                contract='nonnegative direct temporal lerp; no flow; same physical512m domain; positive scale only; half rounding may remove positives but cannot expand support',
                scope='pinned all-time support metadata; no optical/lateral/performance/cloud-family qualification')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    text = json.dumps(support_receipt(), separators=(',', ':'), allow_nan=False)+'\n'
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end='')


if __name__ == '__main__':
    main()
