"""Finite authored optical lifecycle over a pinned detailed volume.

Artist-directed source, not a fluid/thermodynamic simulation. Local density
thresholds expand support; a continuous material-space erosion field opens gaps.
Macro variation is authored and fixed in material coordinates, never frame noise.
The reference is this continuous source function, before temporal cooking.
"""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import numpy as np

SIDE=128
EXTENT=512.0
CODE_MAX=.125
DURATION=240.0
DEFAULT_SHAPE=Path(__file__).resolve().parents[2]/'docs/evidence/cloud-source-2026-11-01/authored-input'


def smooth(x):
    u=np.clip(np.asarray(x,dtype=np.float64),0.,1.)
    # Symmetry avoids cancellation near1: no negative erosion may create fog
    # in empty source space. This is the same quintic, evaluated stably.
    v=np.where(u<=.5,u,1.-u)
    p=v*v*v*(10.+v*(-15.+6.*v))
    return np.where(u<=.5,p,1.-p).astype(np.float32)



class Source:
    def __init__(self,path=DEFAULT_SHAPE):
        self.path=Path(path)
        self.metadata=json.loads((self.path/'shape.json').read_text())
        data=(self.path/'shape.r8').read_bytes()
        self.shape_sha256=hashlib.sha256(data).hexdigest()
        if self.shape_sha256 != self.metadata['product']['sha256']:
            raise ValueError('authored shape hash mismatch')
        self.base=np.frombuffer(data,np.uint8).reshape(SIDE,SIDE,SIDE).astype(np.float32)*(CODE_MAX/255)
        z,y,x=np.mgrid[:SIDE,:SIDE,:SIDE].astype(np.float32)
        q=(np.stack((x,y,z),axis=-1)+.5)*4.
        x,y,z=q[...,0]-256,q[...,1],q[...,2]-256
        height=smooth((y-50.)/170.)
        self.birth=5.+18.*height+8.*(.5+.5*np.sin(x/42.+z/59.))
        # Three smooth, incommensurate, oblique macro modes. Values are bounded
        # by [.0,1.0]; dry regions erode first while neighbouring wet cores persist.
        k1=np.asarray([.81,.29,.51]);k1=k1/np.linalg.norm(k1)*(2*np.pi/95.)
        k2=np.asarray([-.37,.88,.30]);k2=k2/np.linalg.norm(k2)*(2*np.pi/141.)
        k3=np.asarray([.18,-.40,.90]);k3=k3/np.linalg.norm(k3)*(2*np.pi/74.)
        m=.5+.22*np.sin(x*k1[0]+y*k1[1]+z*k1[2]+.2)+.16*np.sin(x*k2[0]+y*k2[1]+z*k2[2]+2.4)+.12*np.cos(x*k3[0]+y*k3[1]+z*k3[2]-1.2)
        self.death=130.+70.*m

    def field(self,s):
        if not 0<=s<=DURATION:
            raise ValueError('source age outside finite lifecycle')
        birth_erosion=CODE_MAX*(1.-smooth((s-self.birth)/55.))
        death_erosion=CODE_MAX*smooth((s-self.death)/40.)
        return np.maximum(self.base-birth_erosion-death_erosion,0).astype(np.float32)

    def descriptor(self):
        return {'source_kind':'authored local optical lifecycle, not meteorology',
                'shape_sha256':self.shape_sha256,
                'source_recipe_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'density_side':SIDE,'extent_m':EXTENT,'duration_s':DURATION,
                'fixed_optical_range_per_m':[0,CODE_MAX],
                'material_velocity_m_per_source_s':[0,0,0],
                'bulk_wind':'external E1 instance trajectory, not baked into local source',
                'birth_onset_range_s':[5,31], 'birth_threshold_duration_s':55,
                'erosion_onset_range_s':[130,200], 'erosion_threshold_duration_s':40,
                'macro_wavelengths_m':[95,141,74],
                'history':'not enabled in this direct-density proof; local creation/erosion must reject reuse',
                'rights':'Derived cloud content CC-BY-SA-3.0; see authored-input/LICENSE.md',
                'reference':'continuous source function sampled directly, not interpolation of cooked frames'}
