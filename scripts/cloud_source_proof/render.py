"""Offline optical diagnostic. Fixed orthographic view, Beer extinction, top light.
This is a content-review instrument, not Thrum V3 or an engine frame-time result.
"""
import argparse
import json
from pathlib import Path
import struct
import sys
import zlib
import numpy as np


def png(path, rgb):
    height, width = rgb.shape[:2]
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    raw = b''.join(b'\0' + row.tobytes() for row in rgb)
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR',struct.pack('>IIBBBBB', width,height,8,2,0,0,0)) + chunk(b'IDAT',zlib.compress(raw,6)) + chunk(b'IEND',b''))


def trilinear(field, xyz, clamp=False):
    """field[Z,Y,X,(C)], xyz[...,3] in voxel-centre index coordinates."""
    size = np.asarray(field.shape[:3][::-1])
    q = np.clip(xyz,0,size-1)
    lo = np.floor(q).astype(np.int32)
    hi = np.minimum(lo+1,size-1)
    f = q-lo
    extra = field.ndim == 4
    result = np.zeros(xyz.shape[:-1] + (field.shape[3:] if extra else ()),np.float32)
    for z in range(2):
        for y in range(2):
            for x in range(2):
                w = (f[...,0] if x else 1-f[...,0])*(f[...,1] if y else 1-f[...,1])*(f[...,2] if z else 1-f[...,2])
                value = field[(hi if z else lo)[...,2],(hi if y else lo)[...,1],(hi if x else lo)[...,0]]
                result += value * (w[...,None] if extra else w)
    if not clamp:
        inside = np.all((xyz>=0)&(xyz<=size-1),axis=-1)
        result *= inside[...,None] if extra else inside
    return result


def optical_image(density, extent=512, width=512, height=384, view='front', silhouette=False):
    # Y-up input. Fixed camera, framing and illumination for every source age.
    side = density.shape[0]
    if view == 'side':
        density = density.transpose(2,1,0)[:,:,::-1]
    cell = extent/side
    tau_sun = np.cumsum(density[:,::-1,:],axis=1)[:,::-1,:]*cell - density*cell*.5
    light = np.exp(-tau_sun*.8)*.75 + np.exp(-tau_sun*.16)*.15 + .13
    yy,xx = np.mgrid[0:height,0:width].astype(np.float32)
    world_x = (xx+.5)/width*384+64
    world_y = 304-(yy+.5)/height*288
    # Rays are aligned with Z: bilinear sampling in XY can be shared by every
    # slice. Vectorize Beer integration; this changes no optical sample values.
    fx=np.clip(world_x[0]/cell-.5,0,side-1)
    fy=np.clip(world_y[:,0]/cell-.5,0,side-1)
    ix=np.floor(fx).astype(np.int32); jx=np.minimum(ix+1,side-1); wx=(fx-ix)[None,None,:]
    iy=np.floor(fy).astype(np.int32); jy=np.minimum(iy+1,side-1); wy=(fy-iy)[None,:,None]
    def plane_sample(field):
        low=field[:,iy[:,None],ix[None,:]]*(1-wx)+field[:,iy[:,None],jx[None,:]]*wx
        high=field[:,jy[:,None],ix[None,:]]*(1-wx)+field[:,jy[:,None],jx[None,:]]*wx
        return (low*(1-wy)+high*wy).astype(np.float32)
    sigma=plane_sample(density)
    shade=plane_sample(light)
    depth=sigma*cell
    before=np.cumsum(depth,axis=0)-depth
    weight=np.exp(-before)*(-np.expm1(-depth))
    brightness=np.sum(weight*shade,axis=0)
    radiance=brightness[...,None]*np.asarray([1.0,.98,.94],np.float32)
    transmittance=np.exp(-np.sum(depth,axis=0))
    if silhouette:
        rgb = np.repeat(transmittance[...,None],3,axis=-1)
    else:
        background = np.zeros_like(radiance)
        background[:] = np.asarray([.24,.40,.64])
        rgb = radiance + transmittance[...,None]*background
    rgb = np.clip(rgb,0,1)
    rgb = np.where(rgb<=.0031308,12.92*rgb,1.055*np.power(rgb,1/2.4)-.055)
    return (rgb*255+.5).astype(np.uint8)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--pilot',type=Path)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else None)
    args.out.mkdir(parents=True,exist_ok=True)
    data=np.load(args.pilot)
    print('INPUT',data.files,flush=True)
    for i,(frame,density) in enumerate(zip(data['frames'],data['density'])):
        sigma=density.transpose(1,0,2)[::-1,:,:]*.15
        for view in ['front','side']:
            for mask in [False,True]:
                dest=args.out/f'{int(frame):03d}-{view}-{"mask" if mask else "lit"}.png'
                png(dest,optical_image(sigma,view=view,silhouette=mask))
        print('RENDERED',int(frame),flush=True)


if __name__=='__main__':
    main()
