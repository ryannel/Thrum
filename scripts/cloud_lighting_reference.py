#!/usr/bin/env python3
"""Independent f64 transport from Metal density snapshots, without Sun caches.

Run the ignored thrum-clouds lighting_stage_snapshots test first. This reads the
stored weather field, not the cached Sun grid, then independently integrates
each Sun ray, the droplet's disc phase, hemisphere irradiance, and view transport.
Only the atmosphere's lent physical radiances/transports are inputs. No cloud
production lighting, phase-table or transport function is called by this script.
"""
import argparse
import array
import csv
import math
from pathlib import Path
import struct


def floats(path):
    result = array.array("f")
    result.frombytes(path.read_bytes())
    return result


def smooth(x):
    x = min(1.0, max(0.0, x))
    return x*x*(3.0-2.0*x)


def random(i, j, k, stream):
    x = (i*0x9e3779b1+j*0x85ebca77+k*0xc2b2ae3d+(stream+131*39)*0x27d4eb2f) & 0xffffffff
    x ^= x >> 15
    x = (x*0x2c1b3c6d) & 0xffffffff
    x ^= x >> 12
    x = (x*0x297a2d39) & 0xffffffff
    x ^= x >> 15
    return x/4294967296.0


def noise(point, scale, stream):
    p = [v/scale for v in point]
    lo = [math.floor(v) for v in p]
    f = [smooth(v-i) for v, i in zip(p, lo)]
    return sum((f[0] if x else 1-f[0]) * (f[1] if y else 1-f[1]) *
               (f[2] if z else 1-f[2]) * random(lo[0]+x, lo[1]+y, lo[2]+z, stream)
               for x in range(2) for y in range(2) for z in range(2))


class Field:
    """f64 reconstruction of the SAME stored weather field and authored noise."""
    def __init__(self, path):
        self.columns = floats(path/'texture-0.bin')
        self.optical = floats(path/'texture-1.bin')
        self.mapping = [list(map(float, l.split())) for l in (path/'mapping.txt').read_text().splitlines()]

    @staticmethod
    def bilinear(data, uv, level):
        p = [v-0.5 for v in uv]
        lo = [math.floor(v) for v in p]
        f = [v-i for v, i in zip(p, lo)]
        result = [0.0]*4
        for x in range(2):
            for y in range(2):
                at = ((level*256+min(255,max(0,lo[1]+y)))*256+min(255,max(0,lo[0]+x)))*4
                w = (f[0] if x else 1-f[0])*(f[1] if y else 1-f[1])
                for j in range(4):
                    result[j] += w*data[at+j]
        return result

    def column(self, point):
        result = [0.0]*4
        for level in range(3,-1,-1):
            x,z,t = self.mapping[level]
            uv = [(point[0]-x)/t,(point[2]-z)/t]
            w = smooth(min(uv[0],uv[1],256-uv[0],256-uv[1])/(256*0.15))
            if not w:
                continue
            c = self.bilinear(self.columns,uv,level)
            o = self.bilinear(self.optical,uv,level)
            thickness = c[1]-c[0]
            value = [0.0]*4
            if thickness>0 and c[0]<point[1]<c[1]:
                h = (point[1]-c[0])/thickness
                vertical = smooth(h/o[2])*(1-smooth((h-(1-o[3]))/o[3]))
                value = [o[1]/thickness*vertical/(1-(o[2]+o[3])*0.5),o[1]*(1-h),o[1]*h,thickness]
            result = [a*(1-w)+b*w for a,b in zip(result,value)]
            if w == 1:
                # Finer levels still replace this value, so do not exit.
                pass
        return result

    def density(self, point, footprint, coarse=False):
        sigma = self.column(point)[0]
        if sigma<=0:
            return 0.0
        signal = min(1,sigma/0.05)
        blend = 0.0 if coarse else min(1,max(0,(footprint-100)/300))
        shape = noise(point,200,40)*(1-blend)+0.5*blend
        blend = 1.0 if coarse else min(1,max(0,footprint/50))
        detail = 0.5 if coarse else noise(point,25,41)*(1-blend)+0.5*blend
        threshold = 0.7*shape
        shaped = max(0,(signal-threshold)/(1-threshold))
        eroded = max(0,shaped-0.7*detail*(1-shaped))
        return sigma*min(1,eroded/signal)

    def sun_depth(self, point, sun, top, footprint, step, coarse=False):
        length = max(0,(top-point[1])/sun[1])
        count = max(1,math.ceil(length/step))
        ds = length/count
        return sum(self.density([p+s*(i+0.5)*ds for p,s in zip(point,sun)],footprint,coarse)*ds for i in range(count))


def phase_reference(ray, sun, diameter, radius):
    """Independent solid-angle midpoint integration of the published fit."""
    gh = math.exp(-0.0990567/(diameter-1.67154))
    gd = math.exp(-2.20679/(diameter+3.91029)-0.428934)
    alpha = math.exp(3.62489-8.29288/(diameter+5.52825))
    w = math.exp(-0.599085/(diameter-0.641583)-0.665888)
    c = min(1,max(-1,sum(a*b for a,b in zip(ray,sun))))
    s = math.sqrt(max(0,1-c*c))

    def hg(mu,g):
        return (1-g*g)/(4*math.pi*(1+g*g-2*g*mu)**1.5)

    first = 0.0
    # Uniform in cos(theta) gives exact solid-angle weights across the disc.
    for i in range(64):
        mu = 1-(i+0.5)/64*(1-math.cos(radius))
        radial = math.sqrt(max(0,1-mu*mu))
        for j in range(128):
            ct = c*mu+s*radial*math.cos(2*math.pi*(j+0.5)/128)
            first += ((1-w)*hg(ct,gh)+w*hg(ct,gd)*(1+alpha*ct*ct)/(1+alpha*(1+2*gd*gd)/3))/(64*128)
    mean = (1-w)*gh+w*gd*(1+alpha*(3+2*gd*gd)/5)/(1+alpha*(1+2*gd*gd)/3)
    return [first]+[hg(c,g) for g in [0.43,0.215,0.1075]],mean


def source(solar, phases, tau, vertical, mean, hemispheres):
    ambient = [sum(hemispheres[h][j]/(1+0.75*(1-mean)*vertical[h]) for h in range(2)) for j in range(3)]
    direct = sum(w*p*math.exp(-b*tau) for w,p,b in zip([0.7,0.2,0.07,0.03],phases,[1,0.5,0.25,0.125]))
    return [s*direct+a for s,a in zip(solar,ambient)],ambient


def analyse(path, sun_step):
    field = Field(path)
    data = floats(path/'buffer-0.bin')
    row = lambda i: list(data[i*4:(i+1)*4])
    camera,ray,sun = row(0)[:3],row(1)[:3],row(2)[:3]
    phases,mean = phase_reference(ray,sun,24,0.00465421)
    hemispheres = [[sum(row(6152+h*8192+i)[j]*row(6152+h*8192+i)[3] for i in range(8192))/8192 for j in range(3)] for h in range(2)]
    prod_hemi = [row(4)[:3],row(5)[:3]]
    details = []
    reference_scatter = [0.0]*3
    replay_scatter = [0.0]*3
    split_scatter = [0.0]*3
    upper_scatter = [0.0]*3
    reference_t = replay_t = 1.0
    n = int(row(7)[0])
    assert n>0, f'{path.name}: ray must intersect occupied cloud'
    # Sampling schedule is production's, so differences isolate lighting from
    # view-step quadrature. A separate finer Sun march measures convergence.
    for i in range(n):
        a = 8+i*8
        point = row(a)[:3]
        at = row(a)[3]
        sigma,tau,step,_ = row(a+1)
        column = row(a+2)
        solar = row(a+3)[:3]
        prod_source = row(a+4)[:3]
        segment = row(a+5)
        air_t,air_s = row(a+6)[:3],row(a+7)[:3]
        density = field.density(point,at/129)
        direct_tau = field.sun_depth(point,sun,row(7)[2],at/129,sun_step)
        reach = min(120,(row(7)[2]-point[1])/max(0.1,sun[1]))
        local_tau = sum(field.density([p+s*(j+0.5)*reach/3 for p,s in zip(point,sun)],at/129)*reach/3 for j in range(3))
        split_tau = local_tau+field.sun_depth([p+s*reach for p,s in zip(point,sun)],sun,row(7)[2],at/129,sun_step,coarse=True)
        ref_source,ambient = source(solar,phases,direct_tau,field.column(point)[1:3],mean,hemispheres)
        replay_source,prod_ambient = source(solar,row(6),tau,column[1:3],row(2)[3],prod_hemi)
        split_source,_ = source(solar,phases,split_tau,field.column(point)[1:3],mean,hemispheres)
        upper_source,_ = source(solar,phases,0.0,field.column(point)[1:3],mean,hemispheres)
        removed = -math.expm1(-density*step)
        for j in range(3):
            reference_scatter[j] += reference_t*removed*(air_t[j]*ref_source[j]+air_s[j])
            split_scatter[j] += reference_t*removed*(air_t[j]*split_source[j]+air_s[j])
            upper_scatter[j] += reference_t*removed*(air_t[j]*upper_source[j]+air_s[j])
            replay_scatter[j] += replay_t*(-math.expm1(-sigma*step))*(air_t[j]*prod_source[j]+air_s[j])
        reference_t *= math.exp(-density*step)
        replay_t *= segment[3]
        details.append(dict(case=path.name,sample=i,distance=at,sigma_production=sigma,sigma_reference=density,
                            sun_T_production=math.exp(-tau),sun_T_reference=math.exp(-direct_tau),
                            sun_T_split_reference=math.exp(-split_tau),
                            phase_production=row(6)[0],phase_reference=phases[0],
                            ambient_production=prod_ambient[1],ambient_reference=ambient[1],
                            source_production=prod_source[1],source_replay=replay_source[1],source_reference=ref_source[1],
                            source_split_reference=split_source[1],
                            segment_S_production=segment[1],segment_S_reference=ref_source[1]*removed,
                            accumulated_S_reference=reference_scatter[1],accumulated_T_reference=reference_t))
        for j,name in enumerate('rgb'):
            details[-1].update({f'ambient_production_{name}':prod_ambient[j],f'ambient_reference_{name}':ambient[j],
                               f'solar_{name}':solar[j],f'source_production_{name}':prod_source[j],
                               f'source_reference_{name}':ref_source[j],f'segment_S_production_{name}':segment[j],
                               f'segment_S_reference_{name}':ref_source[j]*removed,
                               f'accumulated_S_reference_{name}':reference_scatter[j],
                               f'accumulated_pixel_reference_{name}':reference_scatter[j]+reference_t*row(3)[j]})
        for j in range(4):
            details[-1].update({f'phase_production_{j}':row(6)[j],f'phase_reference_{j}':phases[j]})
    texture = (path/'texture-2.bin').read_bytes()
    centre = struct.unpack_from('<4e',texture,((129//2)*129+129//2)*8)
    scale = row(0)[3]
    stored = [v/scale for v in centre[:3]]
    background = row(3)[:3]
    summary = dict(case=path.name,samples=n,phase_production=row(6)[0],phase_reference=phases[0],
                       hemisphere_up_production=prod_hemi[0][1],hemisphere_up_reference=hemispheres[0][1],
                       hemisphere_down_production=prod_hemi[1][1],hemisphere_down_reference=hemispheres[1][1],
                       S_stored=stored[1],S_replayed=replay_scatter[1],S_reference=reference_scatter[1],
                       S_split_reference=split_scatter[1],
                       S_unshadowed_upper_bound=upper_scatter[1],
                       T_stored=centre[3],T_replayed=replay_t,T_reference=reference_t,
                       pixel_production=stored[1]+centre[3]*background[1],
                       pixel_reference=reference_scatter[1]+reference_t*background[1],storage_scale=scale)
    for j,name in enumerate('rgb'):
        summary.update({f'S_stored_{name}':stored[j],f'S_replayed_{name}':replay_scatter[j],
                        f'S_reference_{name}':reference_scatter[j],f'S_split_reference_{name}':split_scatter[j],
                        f'S_unshadowed_upper_bound_{name}':upper_scatter[j],
                        f'pixel_production_{name}':stored[j]+centre[3]*background[j],
                        f'pixel_reference_{name}':reference_scatter[j]+reference_t*background[j]})
        assert abs(stored[j]-replay_scatter[j]) < 0.002*max(1,abs(stored[j])), 'observer replay must match stored transport'
    return details,summary


def write_csv(path, rows):
    with path.open('w',newline='') as f:
        writer = csv.DictWriter(f,fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input',type=Path,default=Path('tmp/clouds-lighting-diagnosis'))
    parser.add_argument('--output',type=Path,default=Path('tmp/clouds-lighting-diagnosis/reference'))
    parser.add_argument('--sun-step',type=float,default=4.0)
    args = parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    samples,pixels = [],[]
    for path in sorted(p for p in args.input.iterdir() if (p/'mapping.txt').exists()):
        a,b = analyse(path,args.sun_step)
        samples.extend(a)
        pixels.append(b)
        print(b,flush=True)
    assert len(pixels)==6, 'six fixed rays required'
    write_csv(args.output/'samples.csv',samples)
    write_csv(args.output/'pixels.csv',pixels)


if __name__ == '__main__':
    main()
