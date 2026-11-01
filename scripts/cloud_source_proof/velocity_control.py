"""Bounded empirical Blender gas UNI advection control, independent of the cooker.

Blender --background --gpu-backend metal --threads 2 --python-exit-code 1
  --python scripts/cloud_source_proof/velocity_control.py -- --out tmp/cloud-source-proof/velocity-control
No runtime or production recipe changes. Writes its own new directory only.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import sys
import time

import bpy
import numpy as np


def read_uni(path):
    with gzip.open(path, 'rb') as stream:
        if stream.read(4) != b'MNT3':
            raise ValueError('Expected MNT3 fullfloat cache')
        header = stream.read(288)
        nx, ny, nz, grid, element, width = struct.unpack_from('<6i', header)
        channels = {1: 1, 2: 3}[element]
        assert width == channels * 4 and struct.unpack_from('<i', header, 276)[0] == 0
        array = np.frombuffer(stream.read(), '<f4')
    assert array.size == nx * ny * nz * channels
    shape = (nz, ny, nx) + ((3,) if channels == 3 else ())
    return array.reshape(shape).copy(), {'shape_zyx': [nz, ny, nx], 'grid_type': grid,
                                         'element_type': element, 'bytes_per_cell': width}


def centered(v, offset):
    out = np.empty_like(v)
    for component, axis in enumerate((2, 1, 0)):
        other = np.take(v[..., component], np.clip(np.arange(v.shape[axis]) + offset,
                                                  0, v.shape[axis] - 1), axis=axis)
        out[..., component] = .5 * (v[..., component] + other)
    return out


def analyze(out, frames, seconds, recipe_hash):
    n = 32
    c = np.arange(n, dtype=np.float64) + .5
    z, y, x = np.meshgrid(c, c, c, indexing='ij')
    positions = np.stack((x, y, z), axis=-1)
    fields, records, headers = [], [], []
    for frame in range(1, frames + 1):
        density, dh = read_uni(out / 'cache/data' / f'density_{frame:04d}.uni')
        velocity, vh = read_uni(out / 'cache/data' / f'vel_{frame:04d}.uni')
        w = np.maximum(density.astype(np.float64) - .0001, 0)
        mass = float(w.sum())
        if mass == 0:
            records.append({'frame': frame, 'tracer_mass_cells': 0, 'centroid_grid_xyz': None,
                            'weighted_raw_velocity_xyz': None, 'tracer_fraction_within_four_cells_of_edge': None})
            fields.append((density, velocity))
            if frame == 1:
                headers = {'density': dh, 'velocity': vh}
            continue
        means = {name: ((field * w[..., None]).sum(axis=(0, 1, 2))/mass).tolist()
                 for name, field in [('forward_face_average', centered(velocity, 1)),
                                     ('same_index', velocity),
                                     ('backward_face_average', centered(velocity, -1))]}
        centroid = (positions * w[..., None]).sum(axis=(0, 1, 2))/mass
        edge = w.copy()
        edge[4:-4, 4:-4, 4:-4] = 0
        mean = np.array(means['forward_face_average'])
        rms = np.sqrt((w[..., None] * (centered(velocity, 1)-mean)**2).sum()/mass)
        records.append({'frame': frame, 'tracer_mass_cells': mass,
                        'relative_weighted_velocity_variation_rms': float(rms/np.linalg.norm(mean)) if np.linalg.norm(mean)>0 else None,
                        'centroid_grid_xyz': centroid.tolist(), 'weighted_raw_velocity_xyz': means,
                        'tracer_fraction_within_four_cells_of_edge': float(edge.sum()/mass)})
        fields.append((density, velocity))
        if frame == 1:
            headers = {'density': dh, 'velocity': vh}
    intervals = []
    for index in range(3, frames - 1):
        a, b = records[index:index+2]
        displacement = np.subtract(b['centroid_grid_xyz'], a['centroid_grid_xyz'])
        models = {}
        for name in a['weighted_raw_velocity_xyz']:
            mean = .5 * (np.array(a['weighted_raw_velocity_xyz'][name]) + b['weighted_raw_velocity_xyz'][name])
            pred = mean * .3125
            models[name] = {'mean_velocity_raw_xyz': mean.tolist(),
                            'predicted_displacement_cells_xyz': pred.tolist(),
                            'observed_over_raw_factor': float(displacement @ mean / (mean @ mean)) if mean @ mean > 0 else None,
                            'relative_vector_error': float(np.linalg.norm(displacement - pred)/np.linalg.norm(displacement)) if np.linalg.norm(displacement)>0 else None}
        intervals.append({'frames': [a['frame'], b['frame']],
                          'observed_displacement_cells_xyz': displacement.tolist(),
                          'relative_mass_change': b['tracer_mass_cells']/a['tracer_mass_cells'] - 1,
                          'models': models})
    report = {
        'schema': 'thrum.cloud-source-proof.velocity-control.v1',
        'producer': {'version': bpy.app.version_string, 'build_hash': bpy.app.build_hash.decode(),
                     'recipe_sha256': recipe_hash},
        'conditions': {'resolution': n, 'domain_side_BU': 32, 'fps': 8, 'time_scale': 1,
                       'manta_internal_units_per_cache_frame': .3125,
                       'injected_velocity_coord_BU_per_timeline_second': [3, -2, 1],
                       'initialization': 'broad18BU cube density .0001/surface band10cells plus7BU sphere density1/surface band1.5cells; both inflows on only frame2',
                       'buoyancy_alpha': 0, 'buoyancy_beta': 0, 'vorticity': 0,
                       'gravity': [0, 0, 0], 'dissolve': False, 'noise': False,
                       'boundary': 'six open borders; no obstacles',
                       'tracer_weight': 'max(exported density-.0001,0)',
                       'first_analyzed_interval': [4, 5], 'timesteps_min_max': [2, 8]},
        'labels_not_empirically_inferred': {
            'source_seconds_per_frame': 1.875,
            'local_metres_per_BU': 16,
            'source_seconds_per_manta_internal_unit': 6,
            'conversion_at_this_32_resolution': 'v_local_m_per_source_s=raw*(32/32)*16/6',
            'conversion_at_production_128_resolution': 'v_local_m_per_source_s=raw*(32/128)*16/6=raw*2/3'},
        'artifact_headers': headers, 'bake_wall_seconds': seconds,
        'records': records, 'intervals': intervals,
        'limits': ['Centroid transport tests signed numerical displacement, not source clock/metric labels.',
                   'Spatially weighted initial flow and pressure create a nonuniform jet; centroid transport error cannot uniquely identify unit scale or face offset.',
                   'One source-frame may include inflow integration; intervals before4 are excluded.',
                   'Semi-Lagrangian/MacCormack transport is not exactly mass conserving; mass and boundary contamination are reported.']}
    selected = [i['models']['forward_face_average'] for i in intervals]
    errors = [i['relative_vector_error'] for i in selected]
    factors = [i['observed_over_raw_factor'] for i in selected]
    report['summary'] = {'mean_empirical_cells_per_raw_velocity_per_frame': float(np.mean(factors)),
                         'expected_cells_per_raw_velocity_per_frame': .3125,
                         'max_relative_vector_error': max(errors),
                         'mean_relative_vector_error': float(np.mean(errors)),
                         'max_absolute_relative_mass_change': max(abs(i['relative_mass_change']) for i in intervals),
                         'max_edge_fraction_analyzed': max(r['tracer_fraction_within_four_cells_of_edge'] for r in records[3:]),
                         'centering_status': 'inconclusive: local jet and scalar transport errors do not independently resolve face offset',
                         'signed_axes_status': 'pass' if all(np.array(i['observed_displacement_cells_xyz']) @ np.array([1,-1,1]) > 0 and all(np.array(i['observed_displacement_cells_xyz'])*[1,-1,1]>0) for i in intervals) else 'failed',
                         'numerical_velocity_scale_status': 'inconclusive',
                         'max_relative_weighted_velocity_variation_rms': max(r['relative_weighted_velocity_variation_rms'] for r in records[3:]),
                         'decision': 'Do not use this local-jet experiment as calibration or independent verification of the .3125 factor.'}
    report['artifact_sha256'] = {str(p.relative_to(out)): hashlib.sha256(p.read_bytes()).hexdigest()
                                for p in sorted((out / 'cache/data').glob('*.uni'))}
    (out / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
    print('VELOCITY_CONTROL_COMPLETE', json.dumps(report['summary']), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = 8
    scene.frame_start, scene.frame_end = 1, 12
    scene.gravity = (0, 0, 0)
    bpy.ops.mesh.primitive_cube_add(size=32, location=(0, 0, 0))
    domain = bpy.context.object
    domain.name = 'passive_advection_control_domain'
    mod = domain.modifiers.new('fluid', 'FLUID')
    mod.fluid_type = 'DOMAIN'
    d = mod.domain_settings
    d.domain_type = 'GAS'
    d.resolution_max = 32
    d.use_adaptive_domain = False
    d.use_noise = False
    d.cache_type = 'MODULAR'
    d.cache_data_format = 'UNI'
    d.cache_directory = str(out / 'cache')
    d.cache_frame_start, d.cache_frame_end = 1, 12
    d.cache_resumable = False
    d.alpha = d.beta = d.vorticity = 0
    d.time_scale = 1
    d.timesteps_min, d.timesteps_max = 2, 8
    d.use_dissolve_smoke = False
    for border in ('front', 'back', 'right', 'left', 'top', 'bottom'):
        setattr(d, 'use_collision_border_' + border, False)
    for broad in (True, False):
        if broad:
            bpy.ops.mesh.primitive_cube_add(size=18)
        else:
            bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=16, radius=7, location=(-1, 0, 0))
        emitter = bpy.context.object
        emitter.name = 'uniform_velocity_seed' if broad else 'dense_passive_tracer'
        fm = emitter.modifiers.new('fluid', 'FLUID')
        fm.fluid_type = 'FLOW'
        flow = fm.flow_settings
        flow.flow_type = 'SMOKE'
        flow.flow_behavior = 'INFLOW'
        flow.surface_distance = 10 if broad else 1.5
        flow.volume_density = 1
        flow.density = .0001 if broad else 1
        flow.temperature = 0
        flow.use_initial_velocity = True
        flow.velocity_coord = (3, -2, 1)
        for frame, active in [(1, False), (2, True), (3, False)]:
            flow.use_inflow = active
            flow.keyframe_insert(data_path='use_inflow', frame=frame)
    scene.frame_set(1)
    bpy.ops.object.select_all(action='DESELECT')
    domain.select_set(True)
    bpy.context.view_layer.objects.active = domain
    bpy.ops.wm.save_as_mainfile(filepath=str(out / 'control.blend'))
    start = time.perf_counter()
    bpy.ops.fluid.bake_data()
    analyze(out, 12, time.perf_counter() - start, hashlib.sha256(Path(__file__).read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
