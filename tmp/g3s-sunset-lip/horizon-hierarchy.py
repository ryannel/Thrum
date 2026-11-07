"""Exact terrain horizon and max-height hierarchy on the showcase's source triangles.

Scratch representation test. Counts node bounds and triangle evaluations, not GPU time.
"""
from bisect import bisect_right
from dataclasses import dataclass
import math
from statistics import median, quantiles
from geometry import camera, cross, eye, image, indices, NX, positions, trace, unit, xs, zs
from terrain_sun_shared import disk

W,H = (1280,720)
TAN_Y = math.tan(math.radians(camera['vertical_fov_degrees']) / 2)
X, Z = len(xs)-1, len(zs)-1
RADIUS = 16384
MIN_DISTANCE = 64.0


@dataclass(frozen=True, slots=True)
class Node:
    x0: int
    z0: int
    x1: int
    z1: int
    ymax: float
    children: tuple


def node(x0, z0, x1, z1):
    if x1 - x0 == z1 - z0 == 1:
        y = max(positions[j*NX+i][1] for i in (x0, x1) for j in (z0, z1))
        return Node(x0, z0, x1, z1, y, ())
    x_edges = (x0, (x0+x1)//2, x1) if x1-x0 > 1 else (x0,x1)
    z_edges = (z0, (z0+z1)//2, z1) if z1-z0 > 1 else (z0,z1)
    children = tuple(node(a,c,b,d) for a,b in zip(x_edges,x_edges[1:])
                     for c,d in zip(z_edges,z_edges[1:]))
    return Node(x0,z0,x1,z1,max(child.ymax for child in children),children)


TREE = node(0,0,X,Z)


def slab(origin, direction, edges, lo, hi):
    for p,d,(a,b) in zip(origin,direction,edges):
        if abs(d) <= 1e-12:
            if not a <= p <= b:
                return
            continue
        one,two = sorted(((a-p)/d,(b-p)/d))
        lo,hi = max(lo,one),min(hi,two)
        if lo >= hi:
            return
    return lo,hi


def edges(n):
    return ((xs[n.x0], xs[n.x1]), (zs[n.z0], zs[n.z1]))


def ratio_bound(n, start, end, receiver_y):
    dy = n.ymax - receiver_y
    return dy / (start if dy >= 0 else end)


def planes_in_cell(x, z, origin, direction, interval):
    """Maximum slope over two exact source triangles clipped in x,-z."""
    row = 6*(z*X+x)
    best = -math.inf
    for off in (0,3):
        a,b,c = (positions[i] for i in indices[row+off:row+off+3])
        # Convert projection to (world x, negative world z) as in the terrain grid.
        bx,bz,cx,cz = b[0]-a[0], a[2]-b[2], c[0]-a[0], a[2]-c[2]
        det = bx*cz-bz*cx
        ox,oz = origin[0]-a[0], origin[2]+a[2]
        u0,u1 = (ox*cz-oz*cx)/det, (direction[0]*cz-direction[1]*cx)/det
        v0,v1 = (bx*oz-bz*ox)/det, (bx*direction[1]-bz*direction[0])/det
        start,end = interval
        for zero, rate in ((u0,u1),(v0,v1),(1-u0-v0,-u1-v1)):
            if abs(rate) < 1e-12:
                if zero < -1e-9:
                    start = end + 1
                continue
            boundary = -zero/rate
            if rate > 0:
                start = max(start,boundary)
            else:
                end = min(end,boundary)
        if start > end + 1e-8:
            continue
        intercept = a[1] + (b[1]-a[1])*u0 + (c[1]-a[1])*v0 - origin[1]
        gradient = (b[1]-a[1])*u1 + (c[1]-a[1])*v1
        # (affine height - receiver) / t is monotone inside a triangle.
        t = start if intercept >= 0 else end
        best=max(best,gradient+intercept/t)
    return best


def cap(origin, direction):
    # Match the existing top-down map around the authored camera and near exclusion.
    return slab((origin[0],origin[2]),direction,
                ((-RADIUS,RADIUS),(-32-RADIUS,-32+RADIUS)),
                MIN_DISTANCE,2*RADIUS)


def reference(origin, direction):
    lo,hi = cap(origin,direction)
    cuts = {lo,hi}
    for coords,p,d in zip((xs,zs),(origin[0],origin[2]),direction):
        if abs(d) > 1e-12:
            cuts.update(t for c in coords if lo < (t := (c-p)/d) < hi)
    cuts = sorted(cuts)
    best = -math.inf
    for a,b in zip(cuts,cuts[1:]):
        t = (a+b)/2
        i = bisect_right(xs,origin[0]+direction[0]*t)-1
        j = bisect_right(zs,origin[2]+direction[1]*t)-1
        if 0 <= i < X and 0 <= j < Z:
            best=max(best,planes_in_cell(i,j,origin,direction,(a,b)))
    return best


def hierarchy(origin, direction, lower, upper, peak_order=True):
    interval = cap(origin,direction)
    planar = origin[0],origin[2]
    best = -math.inf
    checked, bounds, leaves = 1, 0, 0
    stack = [(TREE,interval,ratio_bound(TREE,*interval,origin[1]))]
    while stack:
        n,ab,estimate = stack.pop()
        if estimate <= max(best,lower):
            continue
        if not n.children:
            leaves += 1
            best = max(best,planes_in_cell(n.x0,n.z0,origin,direction,ab))
            if best >= upper:
                break
            continue
        accepted = []
        for child in n.children:
            checked += 1
            overlap=slab(planar,direction,edges(child),*ab)
            if overlap is None:
                continue
            bounds += 1
            estimate=ratio_bound(child,*overlap,origin[1])
            if estimate > max(best,lower):
                accepted.append((child,overlap,estimate))
        # Visit the strongest possible blocker first. Stale bounds are rechecked on pop.
        if peak_order:
            accepted.sort(key=lambda entry: entry[2])
        else:
            accepted.sort(key=lambda entry: entry[1][0],reverse=True)
        stack.extend(accepted)
    return best, (checked,bounds,leaves)


def receiver(x,y,yaw):
    yaw=math.radians(yaw)
    f=(math.sin(yaw),0,-math.cos(yaw))
    r=unit(cross(f,(0,1,0)))
    a=(2*(x+.5)-W)/H*TAN_Y
    b=(1-2*(y+.5)/H)*TAN_Y
    ray=unit((f[0]+a*r[0],b,f[2]+a*r[2]))
    hit=trace(ray)
    if hit is None:
        return
    return eye[0]+ray[0]*hit[0],eye[1]+ray[1]*hit[0],-eye[2]-ray[2]*hit[0]


def sun(hours):
    t=math.tau*(hours/24-.5)
    s=math.sin(t)
    v=math.sqrt(.5)*math.cos(t)
    d=math.hypot(s,v)
    return math.asin(v),(-s/d,v/d)


def stats(data):
    return f'{median(data):.1f}/{quantiles(data,n=100,method="inclusive")[94]:.1f}/{max(data):.1f}'


def run():
    disk_radius=math.radians(.266)
    for hours, yaw in ((6.1,90),(6.6,90),(12,0),(17.4,-90),(17.55,-90),(17.6,-90),
                       (17.6916666666667,-90),(17.8,-90),(17.9,-90),(18.2,-90),
                       (17.69,-110),(17.69,-70)):
        el,direction=sun(hours)
        lower,upper=(math.tan(el+side*disk_radius) for side in (-1,1))
        samples=[]
        for y in range(382,649,17):
            for x in range(96,W,96):
                origin=receiver(x,y,yaw)
                if origin is None or cap(origin,direction) is None:
                    continue
                want=disk(el,math.atan(reference(origin,direction)))
                row=[]
                for order in (True,False):
                    got,count=hierarchy(origin,direction,lower,upper,order)
                    visible=disk(el,math.atan(max(lower,got)))
                    assert abs(want-visible)<1e-7,(hours,yaw,x,y,visible,want)
                    row.append(count)
                samples.append(row)
        peak,near=zip(*samples)
        print(f'{hours:7.4f}h {yaw:4}° n={len(samples):3} | '
              f'peak-order tests {stats([r[0] for r in peak])}, bounds {stats([r[1] for r in peak])}, leaves {stats([r[2] for r in peak])} | '
              f'near-order tests {stats([r[0] for r in near])}, bounds {stats([r[1] for r in near])}, leaves {stats([r[2] for r in near])}', flush=True)
    origin=receiver(640,434,-90)
    el,direction=sun(17.4+35/120)
    horizon=reference(origin,direction)
    print('witness',math.degrees(math.atan(horizon)),disk(el,math.atan(horizon)),
          hierarchy(origin,direction,math.tan(el-disk_radius),math.tan(el+disk_radius)), flush=True)


if __name__ == '__main__':
    run()
