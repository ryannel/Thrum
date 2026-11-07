"""Adversarial positive-length cell intersection checks with an exact sparse oracle."""
from array import array
from pathlib import Path
import math
import struct

root = Path(__file__).resolve().parent/'sparse-run'
root.mkdir(exist_ok=True)
width = 1024
radius = math.atan(.004654244)
cells = {
    (1,0): 8, (2,1): 8, (8,3): 15, (511,511): 31, (513,512): 32,
    (1023,2): 18, (1022,1): 18, (600,600): 24, (601,599): 27,
    (4,730): 7, (512,730): 7, (818,730): 7,
}
raster = array('f',[0])*width**2
for (x,y), height in cells.items():
    raster[y*width+x] = (height + 32)/256
with (root/'raster.bin').open('wb') as file:
    raster.tofile(file)

def f32(value):
    return struct.unpack('<f', struct.pack('<f',value))[0]

def span(x, y, ox, oy, dx, dy):
    lo,hi = 64.0, 32768.0
    for i,p,d in ((x,ox,dx),(y,oy,dy)):
        if d == 0:
            if not i <= p < i+1:
                return
        else:
            a,b = sorted(((i-p)/d,(i+1-p)/d))
            lo,hi = max(lo,a),min(hi,b)
    if lo < hi:
        return lo,hi

def fraction(el, slope):
    c = min(1,max(-1,(el-math.atan(slope))/radius))
    return .5+(c*math.sqrt(1-c*c)+math.asin(c))/math.pi

rays, expected, labels = array('f'), array('f'), []
def add(label, ox, oy, dx, dy, receiver, elevation):
    length = math.hypot(dx,dy)*32
    el = math.radians(elevation)
    ox,oy,dx,dy,receiver,el = map(f32,(ox,oy,dx/length,dy/length,receiver,el))
    lower,upper = math.tan(el-radius),math.tan(el+radius)
    best = lower
    for (x,y), height in cells.items():
        interval = span(x,y,ox,oy,dx,dy)
        if interval:
            rise = height - receiver
            best = max(best,rise/interval[rise<0])
    rays.extend((ox,oy,dx,dy,receiver,el,256,-32,lower,upper,32,0))
    expected.extend((fraction(el,best),fraction(el,best),0,0,len(labels),0))
    labels.append(label)

for edge in (-.001,0,.001):
    add(f'cardinal east edge {edge}',-4,1+edge,1,0,6,1)
    add(f'cardinal west edge {edge}',1031,2+edge,-1,0,13,1)
    add(f'cardinal negative slope {edge}',0,730+edge,1,0,20,-.06)
    add(f'near diagonal sliver {edge}',-4,-4+edge,1,1,6,.2)
    add(f'far diagonal sliver {edge}',1000,1000+edge,-1,-1,34,-.01)
    add(f'counter diagonal {edge}',580,620+edge,1,-1,19,.1)
    add(f'map exit {edge}',1023.99,edge,1,0,0,0)
    add(f'axis at map upper boundary {edge}',1024+edge,-2,0,1,14,1)
add('empty interior',100,200,-1,0,12,0)
add('empty outside',-12,-20,-1,0,12,0)
add('first two cells excluded',0,0.5,1,0,7,0)
add('far receiver over higher blocker',500,730.5,1,0,6,.025)
add('far receiver over lower blocker',500,730.5,1,0,8,-.025)
add('exact corner leaves zero overlap',0,0,1,1,22,0)
assert any(e == 0 for e in expected[::6]) and any(0 < e < 1 for e in expected[::6])
assert any(e == 1 for e in expected[::6])
for name,values in (('rays',rays),('expected',expected)):
    with (root/f'{name}.bin').open('wb') as file:
        values.tofile(file)
(root/'cases.txt').write_text(''.join(f'{i}: {label}\n' for i,label in enumerate(labels)))
