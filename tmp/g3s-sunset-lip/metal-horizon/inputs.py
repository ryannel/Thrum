"""Write the actual spur height raster and exact/flat ray referees for the Metal probe."""
from array import array
import math
import runpy
from pathlib import Path
import sys

out = Path(__file__).resolve().parent
sys.path.insert(0, str(out.parent))
scope = runpy.run_path(str(out.parent / 'horizon-raster.py'))
N, S, X0, Z0, W, evaluate, heights, pyramid, receiver, sun, reference, cap, positions = (
    scope[k] for k in 'N S X0 Z0 W evaluate heights pyramid receiver sun reference cap positions'.split()
)

assert __import__('os').environ.get('THRUM_HORIZON_MODEL') == 'flat'
height = heights()
levels = pyramid(height)
low = min(v[1] for v in positions) - 1
high = max(v[1] for v in positions) + 1
scale = high - low
array('f', ((height[z*(N+1)+x] - low) / scale for z in range(N) for x in range(N))).tofile(open(out/'raster.bin', 'wb'))
rays, expected = array('f'), array('f')
radius = math.atan(.004654244)
def disk(el, horizon):
    c = min(1, max(-1, (el-horizon)/radius))
    return .5 + (c*math.sqrt(1-c*c)+math.asin(c))/math.pi
for hours, yaw in ((6.1,90),(6.6,90),(12,0),(17.4,-90),(17.55,-90),(17.6,-90),
                   (17.4+35/120,-90),(17.8,-90),(17.9,-90),(18.2,-90),
                   (17.69,-110),(17.69,-70)):
    el, direction = sun(hours)
    coords = [(x,y) for y in range(382,649,17) for x in range(96,W,96)]
    if hours == 17.4+35/120:
        coords.extend((x,434) for x in (320,640,960))
    for x,y in coords:
        origin = receiver(x,y,yaw)
        if origin is None or cap(origin,direction) is None:
            continue
        lower,upper = math.tan(el-radius),math.tan(min(el+radius,1.5))
        result,_,_,_ = evaluate(height,levels,origin,direction,lower,upper)
        rays.extend(((origin[0]-X0+S/2)/S, (origin[2]-Z0+S/2)/S, *[d/S for d in direction],
                     origin[1], el, scale, low, lower, upper, high-1, 0))
        expected.extend((disk(el,math.atan(max(result,lower))), disk(el,math.atan(reference(origin,direction))),
                         hours,yaw,x,y))
rays.tofile(open(out/'rays.bin', 'wb'))
expected.tofile(open(out/'expected.bin', 'wb'))
print(f'{len(rays)//12} rays, Depth32 affine range {low:.3f} to {high:.3f} m')
