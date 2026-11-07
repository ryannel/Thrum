"""Budget/error probe for gap-free traversal over the existing 32 m height raster.

Leaf heights are sampled from the actual terrain and reconstructed bilinearly.
This reproduces a representation, not GPU bandwidth, depth encoding, or dynamic casters.
"""
from array import array
from bisect import bisect_right
import math
from pathlib import Path
import runpy
import os
from geometry import NX, indices, positions, xs, zs

p=runpy.run_path(str(Path(__file__).with_name('horizon-hierarchy.py')))
slab, receiver, sun, disk, stats, reference = (p[name] for name in ('slab','receiver','sun','disk','stats','reference'))
cap = p['cap']
R=16384.0
N=1024
S=32.0
# Top-down orientation and texel centres match the authored eye's snapped view.
X0,Z0=-R+S/2,-R-32+S/2
W=p['W']
MODE=os.environ.get('THRUM_HORIZON_MODEL','bilinear')
FLAT=MODE.startswith('flat')
BIAS=4 if MODE=='flat-bias' else 0
assert MODE in ('bilinear','flat','flat-bias')


def coordinate(values, start, count):
    result=[]
    for i in range(count):
        q=start+min(i,N-1)*S
        ix=bisect_right(values,q)-1
        result.append((ix,(q-values[ix])/(values[ix+1]-values[ix])))
    return result


def heights():
    xw=coordinate(xs,X0,N+1)
    zw=coordinate(zs,Z0,N+1)
    height=tuple(v[1] for v in positions)
    raster=array('f')
    for z,v in zw:
        base=z*NX
        index_base=6*z*(NX-1)
        for x,u in xw:
            a=height[base+x]
            b=height[base+x+1]
            c=height[base+x+NX]
            d=height[base+x+NX+1]
            tri=indices[index_base+6*x:index_base+6*x+3]
            if base+x+NX+1 in tri:
                value=(a+(b-a)*u+(d-b)*v if u>=v else a+(d-c)*u+(c-a)*v)
            else:
                value=(a+(b-a)*u+(c-a)*v if u+v<=1 else d+(d-c)*(u-1)+(d-b)*(v-1))
            raster.append(value)
    return raster


def pyramid(height):
    w=N+1
    level=array('f',(max(height[j*w+i],height[j*w+i+1],height[(j+1)*w+i],height[(j+1)*w+i+1])
                     for j in range(N) for i in range(N)))
    if FLAT:
        level=array('f',(height[j*w+i] for j in range(N) for i in range(N)))
    levels=[level]
    n=N
    while n>1:
        level=array('f',(max(level[j*n+i],level[j*n+i+1],level[(j+1)*n+i],level[(j+1)*n+i+1])
                         for j in range(0,n,2) for i in range(0,n,2)))
        levels.append(level)
        n//=2
    return levels


def leaf(height,x,z,origin,direction,interval):
    w=N+1
    if FLAT:
        dy=height[z*w+x]-origin[1]-BIAS
        return dy/(interval[0] if dy>=0 else interval[1])
    a,b=height[z*w+x:z*w+x+2]
    c,d=height[(z+1)*w+x:(z+1)*w+x+2]
    u=(origin[0]-(X0+x*S))/S
    v=(origin[2]-(Z0+z*S))/S
    dx,dz=(d/S for d in direction)
    mixed=d-b-c+a
    k0=a+(b-a)*u+(c-a)*v+mixed*u*v-origin[1]
    k1=(b-a)*dx+(c-a)*dz+mixed*(u*dz+v*dx)
    k2=mixed*dx*dz
    lo,hi=interval
    result=max(k0/lo+k1+k2*lo,k0/hi+k1+k2*hi)
    if k0<0 and k2<0:
        peak=math.sqrt(k0/k2)
        if lo<peak<hi:
            result=max(result,k0/peak+k1+k2*peak)
    return result


def region(level,x,z):
    step=S*(1<<level)
    offset=S/2 if FLAT else 0
    return ((X0+x*step-offset,X0+(x+1)*step-offset),
            (Z0+z*step-offset,Z0+(z+1)*step-offset))


def evaluate(height,levels,origin,direction,lower,upper):
    root=len(levels)-1
    target=slab((origin[0],origin[2]),direction,region(root,0,0),*cap(origin,direction))
    if target is None:
        return -math.inf,0,0,0
    planar=origin[0],origin[2]
    best=-math.inf
    checks,reads,leaves=1,1,0
    stack=[(root,0,0,target,math.inf)]
    while stack:
        level,x,z,span,estimate=stack.pop()
        if estimate<=max(lower,best):
            continue
        if level==0:
            leaves+=1
            best=max(best,leaf(height,x,z,origin,direction,span))
            if best>=upper:
                break
            continue
        smaller=level-1
        width=N>>smaller
        children=[]
        for j in (2*z,2*z+1):
            for i in (2*x,2*x+1):
                checks+=1
                ab=slab(planar,direction,region(smaller,i,j),*span)
                if ab is None:
                    continue
                reads+=1
                dy=levels[smaller][j*width+i]-origin[1]-BIAS
                bound=dy/(ab[0] if dy>=0 else ab[1])
                if bound>max(lower,best):
                    children.append((smaller,i,j,ab,bound))
        children.sort(key=lambda child: child[4])
        stack.extend(children)
    return best,checks,reads,leaves


def coarse(height,origin,direction,lower,upper,ceiling):
    best = -math.inf
    count = 0
    lo,hi=cap(origin,direction)
    for i in range(96):
        distance=64*512**(i/95)
        if distance>=hi or best>=upper or ceiling-origin[1]<=max(lower,best)*distance:
            break
        a=math.floor((origin[0]+direction[0]*distance-X0)/S+.5)
        b=math.floor((origin[2]+direction[1]*distance-Z0)/S+.5)
        if not (0<=a<N and 0<=b<N):
            break
        count+=1
        best=max(best,(height[b*(N+1)+a]-origin[1]-4)/distance)
    return best,count


def run():
    height=heights()
    levels=pyramid(height)
    print(f'{MODE}: reconstructed grid {len(height)} vertices, {sum(len(level) for level in levels)} max cells',flush=True)
    rad=math.radians(.266)
    ceiling=max(v[1] for v in positions)
    for hours,yaw in ((6.1,90),(6.6,90),(12,0),(17.4,-90),(17.55,-90),(17.6,-90),
                      (17.6916666666667,-90),(17.8,-90),(17.9,-90),(18.2,-90),
                      (17.69,-110),(17.69,-70)):
        el,direction=sun(hours)
        lower,upper=math.tan(el-rad),math.tan(el+rad)
        samples=[]
        for y in range(382,649,17):
            for x in range(96,W,96):
                origin=receiver(x,y,yaw)
                if origin is None or cap(origin,direction) is None:
                    continue
                want=disk(el,math.atan(reference(origin,direction)))
                got,checks,reads,leaves=evaluate(height,levels,origin,direction,lower,upper)
                error=disk(el,math.atan(max(got,lower)))-want
                before,taps=coarse(height,origin,direction,lower,upper,ceiling)
                old_error=disk(el,math.atan(max(before,lower)))-want
                samples.append((checks,reads,leaves,100*abs(error),100*max(0,error),taps,
                                100*abs(old_error),100*max(0,old_error),reads+(1 if FLAT else 4)*leaves))
        print(f'{hours:7.4f}h {yaw:4}° n={len(samples):3} | tests {stats([r[0] for r in samples])}, '
              f'bounds {stats([r[1] for r in samples])}, leaves {stats([r[2] for r in samples])}, '
              f'abs error pp {stats([r[3] for r in samples])}, light leak pp {stats([r[4] for r in samples])} | '
              f'coarse taps {stats([r[5] for r in samples])}, abs error pp {stats([r[6] for r in samples])}, '
              f'leak pp {stats([r[7] for r in samples])}, raster reads (bounds+leaf loads) {stats([r[8] for r in samples])}',flush=True)
    el,direction=sun(17.4+35/120)
    for x in (320,640,960):
        origin=receiver(x,434,-90)
        got,checks,reads,leaves=evaluate(height,levels,origin,direction,math.tan(el-rad),math.tan(el+rad))
        print(f'witness {x}/434: {math.degrees(math.atan(got)):.4f}°, disk {disk(el,math.atan(got)):.4f}, '
              f'{checks} tests / {reads} bounds / {leaves} leaves',flush=True)


if __name__=='__main__':
    run()
