"""Prepare one pinned official WDAS density shape, with authored scale/optics.

Blender --background --python prepare_shape.py -- --input <extracted
wdas_cloud/wdas_cloud_quarter.vdb> --out <new-authored-input-directory>
The public ZIP/member/hash below own recovery; no local cache is a dependency.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

import numpy as np
import openvdb
import bpy

SOURCE_SHA256 = '279efd7566f4f8f3d05a1cb4529b7e903d64a6b4e91c2edce5824b9b5c9f722a'
SOURCE_PAGE = 'https://www.disneyanimation.com/resources/clouds/'
SOURCE_ZIP = 'https://assets.disneyanimation.com/wdas_cloud.zip'
SOURCE_MEMBER = 'wdas_cloud/wdas_cloud_quarter.vdb'
LICENSE_URL = 'https://media.disneyanimation.com/uploads/production/data_set_asset/6/asset/License_Cloud.pdf'
LICENSE_TEXT = '''# WDAS cloud shape attribution and license

The source cloud dataset is Copyright 2017 Disney Enterprises, Inc., licensed
under [Creative Commons Attribution-ShareAlike 3.0 Unported](https://creativecommons.org/licenses/by-sa/3.0/).
The reference photograph is by Kevin Udy, Colorado Clouds Blog,
[26 July 2012 cumulus photograph](https://coclouds.com/436/cumulus/2012-07-26/),
also under CC BY-SA 3.0.

Source: [Walt Disney Animation Studios Cloud Data Set](https://www.disneyanimation.com/resources/clouds/).
Official [dataset license notice](https://media.disneyanimation.com/uploads/production/data_set_asset/6/asset/License_Cloud.pdf).

`shape.r8` is a Thrum-authored adaptation of the quarter-resolution density grid:
uniform geometry scale 0.6, translated to a fixed cube, sampled at 128 cubed,
with a fixed authored optical conversion and R8 quantization. These are changes
to the original; this is not the original dataset or its physical-unit calibration.
The derived shape and its accompanying shape metadata are distributed under
CC BY-SA 3.0. Preserve this attribution and the source/license links when sharing
the adaptation. No endorsement by Disney or the photographer is implied.

The generating Python recipe is separate tooling; the dataset license governs
the adapted density asset and its descriptive metadata.
'''


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def trilinear(array, x, y, z):
    """Independent XYZ source sampler; zero background outside the native crop."""
    positions = (x, y, z)
    lower = [np.floor(p).astype(np.int32) for p in positions]
    fractions = [p-lo for p, lo in zip(positions, lower)]
    corners = {}
    for dx in (0, 1):
        for dy in (0, 1):
            for dz in (0, 1):
                corner = (dx, dy, dz)
                indices = [lo+d for lo, d in zip(lower, corner)]
                valid = np.ones_like(x, dtype=bool)
                for axis, index in enumerate(indices):
                    valid &= (index >= 0) & (index < array.shape[axis])
                safe = [np.clip(i, 0, array.shape[a]-1) for a, i in enumerate(indices)]
                corners[corner] = np.where(valid, array[tuple(safe)], 0).astype(np.float64)
    def lerp(a, b, t):
        return a+(b-a)*t
    along_x = {(y, z): lerp(corners[0, y, z], corners[1, y, z], fractions[0])
               for y in (0, 1) for z in (0, 1)}
    along_y = {z: lerp(along_x[0, z], along_x[1, z], fractions[1]) for z in (0, 1)}
    return lerp(along_y[0], along_y[1], fractions[2])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index('--')+1:])
    if digest(args.input) != SOURCE_SHA256:
        raise ValueError('input is not the pinned official WDAS quarter cloud member')
    if args.out.exists() and any(args.out.iterdir()):
        raise ValueError('output must be new or empty')
    grid = openvdb.read(str(args.input), 'density')
    minimum, maximum = grid.evalMinMax()
    if minimum < 0 or maximum > 1:
        raise ValueError('source density outside[0,1]; fixed optical gain cannot encode it without clipping')
    low, high = [np.asarray(v, np.int32) for v in grid.evalActiveVoxelBoundingBox()]
    dimensions = high-low+1
    origin = np.asarray(grid.transform.indexToWorld((0, 0, 0)), np.float64)
    basis = np.asarray([grid.transform.indexToWorld(tuple(p)) for p in np.eye(3)], np.float64)-origin
    diagonal = np.diag(basis)
    if not np.allclose(basis, np.diag(diagonal), atol=1e-12) or np.any(diagonal <= 0):
        raise ValueError('expected positive axis-aligned official source transform')
    source_center = origin+.5*(low+high)*diagonal
    target_center = np.asarray([256., 144., 256.])
    geometry_scale, side, extent, code_max = .6, 128, 512., .125
    # OpenVDB Python copy arrays addressXYZ, independently of the outputZYX layout.
    source = np.empty(tuple(int(v) for v in dimensions), np.float32)
    grid.copyToArray(source, ijk=tuple(int(v) for v in low))
    output = np.empty((side, side, side), np.uint8)
    coordinates = (np.arange(side, dtype=np.float64)+.5)*extent/side
    yy, xx = np.meshgrid(coordinates, coordinates, indexing='ij')
    max_sigma, quant_error = 0., 0.
    for zi, world_z in enumerate(coordinates):
        world = (xx, yy, np.full_like(xx, world_z))
        index = [((world[a]-target_center[a])/geometry_scale+source_center[a]-origin[a])/
                 diagonal[a]-low[a] for a in range(3)]
        rho = trilinear(source, *index)
        sigma = .125*rho
        if np.any(sigma < 0) or float(sigma.max()) > code_max:
            raise ValueError('fixed authored optical range exceeded during resampling')
        code = np.rint(sigma/code_max*255).astype(np.uint8)
        output[zi] = code
        max_sigma = max(max_sigma, float(sigma.max()))
        quant_error = max(quant_error, float(np.abs(code.astype(np.float64)*code_max/255-sigma).max()))
    args.out.mkdir(parents=True, exist_ok=True)
    shape_path = args.out/'shape.r8'
    shape_path.write_bytes(output.tobytes(order='C'))
    license_path = args.out/'LICENSE.md'
    license_path.write_text(LICENSE_TEXT)
    support = np.argwhere(output > 0)
    metadata = {
        'schema': 'thrum.cloud-source-proof.authored-shape.v1',
        'id': 'thrum.authored-shape.wdas-quarter.scale-0_6.v1',
        'role': 'fixed-precision authored shape master, not runtime promotion or physical Disney-unit preservation',
        'source': {'title': 'Walt Disney Animation Studios Cloud Data Set',
            'official_page': SOURCE_PAGE, 'public_archive': SOURCE_ZIP, 'archive_member': SOURCE_MEMBER,
            'member_sha256': SOURCE_SHA256, 'channel': 'density',
            'rights': 'Copyright2017DisneyEnterprisesInc.;CC-BY-SA-3.0',
            'reference_photo_creator': 'Kevin Udy',
            'reference_photo': 'https://coclouds.com/436/cumulus/2012-07-26/',
            'official_license_notice': LICENSE_URL,
            'value_range': [float(minimum), float(maximum)],
            'index_active_bbox_min_xyz': low.tolist(), 'index_active_bbox_max_xyz': high.tolist(),
            'index_to_world_origin_xyz': origin.tolist(), 'index_to_world_basis_xyz': basis.tolist(),
            'active_voxel_dimensions_xyz': dimensions.tolist(),
            'active_cell_bounds_extent_original_units_xyz': (dimensions*diagonal).tolist(),
            'active_cell_bounds_center_original_units_xyz': source_center.tolist(),
            'recovery': 'Obtain official ZIP, extract exact member and verify pinned member hash; cached local path is not a dependency. Fresh bulk archive download not performed in this proof.'},
        'authored_transform': {'source_axes': 'Houdini right-handedY-upXYZ; preserved',
            'uniform_geometry_scale': geometry_scale,
            'source_center_maps_to_local_metres_xyz': target_center.tolist(),
            'scaled_active_cell_bounds_extent_metres_xyz': (dimensions*diagonal*geometry_scale).tolist(),
            'optical_conversion': 'sigma_shape_m^-1 =0.125*max(original_density,0), fixed; no peak or per-frame normalization',
            'sampling': 'independent trilinear source-density sampling at target voxel centers; zero background',
            'physical_limit': 'geometry and extinction are explicitly authored; inverse-size optical-depth preservation is not claimed'},
        'product': {'file': 'shape.r8', 'sha256': digest(shape_path), 'bytes': shape_path.stat().st_size,
            'side': side, 'extent_m': extent, 'bounds_min_xyz_m': [0, 0, 0],
            'axes': 'right-handedY-upXYZ', 'storage': 'R8_UNORM; rawuint8; Xfastest thenYthenZ; array[Z,Y,X]',
            'runtime_sigma_max_per_m': code_max, 'decode': 'sigma_m^-1 =code/255*0.125',
            'observed_prequantization_max_sigma_per_m': max_sigma,
            'max_nodal_quantization_error_per_m': quant_error,
            'nonzero_voxels': int(len(support)),
            'nonzero_index_bbox_min_zyx': support.min(axis=0).tolist(),
            'nonzero_index_bbox_max_zyx': support.max(axis=0).tolist()},
        'license': {'file': 'LICENSE.md', 'sha256': digest(license_path),
                    'derived_asset_license': 'CC-BY-SA-3.0',
                    'url': 'https://creativecommons.org/licenses/by-sa/3.0/'},
        'rebuild': {'recipe': 'scripts/cloud_source_proof/prepare_shape.py',
            'recipe_sha256': digest(__file__), 'blender_version': bpy.app.version_string,
            'blender_build': bpy.app.build_hash.decode(), 'numpy_version': np.__version__,
            'peak_source_dense_bytes': int(source.nbytes),
            'command': 'blender --background --python scripts/cloud_source_proof/prepare_shape.py -- --input <extracted/wdas_cloud/wdas_cloud_quarter.vdb> --out <new-authored-input-directory>'},
        'validation': 'range/hash/transform/layout checks passed; static body appearance remains for visual review'}
    (args.out/'shape.json').write_text(json.dumps(metadata, indent=2)+'\n')
    print('SHAPE_COMPLETE', json.dumps(metadata['product']), flush=True)


if __name__ == '__main__':
    main()
