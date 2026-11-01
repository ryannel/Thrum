"""Rebuild a bounded authored optical puff master using Blender's Mantaflow.
Not condensed water, atmospheric microphysics, or a runtime engine dependency.
Run with Blender --background --gpu-backend metal --python-exit-code 1 --python ... -- ...
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import time

import bpy


def arguments():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--resolution', type=int, default=64)
    parser.add_argument('--frames', type=int, default=129)
    return parser.parse_args(sys.argv[sys.argv.index('--') + 1:])


def main():
    args = arguments()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = 8
    scene.frame_start = 1
    scene.frame_end = args.frames
    scene.gravity = (0, 0, -9.81)
    # Blender metres map to 16 optical local metres. Eight solver frames per
    # second map to 15 source-seconds per solver-second; no world wind here.
    bpy.ops.mesh.primitive_cube_add(size=32, location=(0, 0, 16))
    domain = bpy.context.object
    domain.name = 'optical_puff_domain'
    modifier = domain.modifiers.new('fluid', 'FLUID')
    modifier.fluid_type = 'DOMAIN'
    settings = modifier.domain_settings
    settings.domain_type = 'GAS'
    settings.resolution_max = args.resolution
    settings.use_adaptive_domain = False
    settings.use_noise = False
    settings.cache_type = 'MODULAR'
    settings.cache_data_format = 'UNI'
    settings.cache_directory = str(out / 'cache')
    settings.cache_frame_start = 1
    settings.cache_frame_end = args.frames
    settings.cache_resumable = False
    settings.openvdb_cache_compress_type = 'ZIP'
    settings.alpha = 0.0
    settings.beta = 0.35
    settings.vorticity = 0.55
    settings.time_scale = 1.0
    settings.timesteps_min = 2
    settings.timesteps_max = 8
    settings.use_dissolve_smoke = True
    settings.use_dissolve_smoke_log = False
    settings.dissolve_speed = 55
    emitters = [(-2.6, -0.7, 6.0, 2.0, 2, 21),
                (0.2, 0.4, 6.4, 2.4, 10, 30),
                (2.9, -0.2, 7.0, 1.8, 20, 39)]
    for index, (x, y, z, radius, start, stop) in enumerate(emitters):
        bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=16, radius=radius, location=(x, y, z))
        emitter = bpy.context.object
        emitter.name = f'finite_inflow_{index}'
        flowmod = emitter.modifiers.new('fluid', 'FLUID')
        flowmod.fluid_type = 'FLOW'
        flow = flowmod.flow_settings
        flow.flow_type = 'SMOKE'
        flow.flow_behavior = 'INFLOW'
        flow.surface_distance = 1.0
        flow.volume_density = 1.0
        flow.density = 0.8
        flow.temperature = 0.7
        flow.use_initial_velocity = True
        flow.velocity_coord = (0.12 * (index - 1), 0.08 * index, 0.65)
        for frame, active in [(1, False), (start, True), (stop, False)]:
            flow.use_inflow = active
            flow.keyframe_insert(data_path='use_inflow', frame=frame)
        emitter.hide_render = True
    scene.frame_set(1)
    bpy.ops.object.select_all(action='DESELECT')
    domain.select_set(True)
    bpy.context.view_layer.objects.active = domain
    scene.render.engine = 'CYCLES'
    bpy.ops.wm.save_as_mainfile(filepath=str(out / 'master.blend'))
    begin = time.perf_counter()
    bpy.ops.fluid.bake_data()
    elapsed = time.perf_counter() - begin
    metadata = {
        'id': 'thrum.authored-optical-puff.v1',
        'producer': 'Blender Mantaflow gas, authored optical effect; not condensed water',
        'blender_version': bpy.app.version_string,
        'blender_build': bpy.app.build_hash.decode(),
        'recipe_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'resolution': args.resolution, 'frames': args.frames,
        'solver_fps': 8, 'source_seconds_per_blender_timeline_second': 15,
        'source_seconds_per_manta_internal_time': 6,
        'source_seconds_per_cache_frame': 1.875,
        'frame_one_source_age_s': 0,
        'cache_format': 'UNI full float32, full-grid velocity; VDB density-clipped velocity rejected',
        'local_metres_per_blender_metre': 16, 'domain_side_local_m': 512,
        'axes': 'Blender Z-up; cooker maps (x,z,-y) to right-handed Y-up',
        'bulk_drift_local_m_per_source_s': [0,0,0],
        'optical_conversion': 'sigma_local_m_inv = 0.15 * max(density,0), fixed for entire source',
        'bake_wall_s': elapsed,
        'rights': 'Original recipe and generated effect for Thrum; no third-party source asset',
        'parameters': {'alpha': 0.0, 'beta': 0.35, 'vorticity': 0.55, 'linear_dissolve_frames': 55, 'temperature': 0.7, 'density': 0.8, 'emitters': emitters},
        'limits': 'Dissipating gas effect; no water/energy conservation or meteorological realism claim',
    }
    (out / 'source.json').write_text(json.dumps(metadata, indent=2) + '\n')
    print('SOURCE_BAKE_COMPLETE', json.dumps(metadata), flush=True)


if __name__ == '__main__':
    main()
