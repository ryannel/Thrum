#!/usr/bin/env python3
"""Authored dry-valley showcase, stdlib only. Run beside this file; no engine/cooker."""
import hashlib
import argparse
import bisect
import json
import math
import platform
import random
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent
EVIDENCE = Path(__file__).resolve().parents[4] / 'tmp/g3s-showcase/geometry-v2'
NX, NZ = 161, 185
# Ground albedo: one tileable texture repeated in world space around the fixed camera's foot.
GROUND_TEXTURE_SIZE = 512
GROUND_SYNTH_SIZE = 1024         # built at twice the shipped size, then box-filtered in linear light
GROUND_TILE_METRES = 16.          # one repeat near the camera: 1.56 cm per texel
GROUND_TILE_CENTRE = (0., 16.)    # camera x, z
GROUND_TILE_GROWTH_RADIUS = 40.   # beyond this the repeat grows as distance**GROUND_TILE_GROWTH
GROUND_TILE_GROWTH = .5


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def cross(a, b):
    return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])


def dot(a, b):
    return sum(x*y for x, y in zip(a, b))


def unit(a):
    s = math.sqrt(dot(a, a))
    assert s > 1e-12
    return tuple(x/s for x in a)


def hash2(x, z):
    n = ((x * 374761393) ^ (z * 668265263) ^ 39059) & 0xffffffff
    n = ((n ^ (n >> 13)) * 1274126177) & 0xffffffff
    return ((n ^ (n >> 16)) & 0xffffffff) / 4294967295 * 2 - 1


def noise(x, z, scale):
    x, z = x / scale, z / scale
    a, b = math.floor(x), math.floor(z)
    u, v = x-a, z-b
    u, v = u*u*(3-2*u), v*v*(3-2*v)
    p = hash2(a,b)*(1-u)+hash2(a+1,b)*u
    q = hash2(a,b+1)*(1-u)+hash2(a+1,b+1)*u
    return p*(1-v)+q*v


def hill(x, z, cx, cz, rx, rz, height):
    return height * math.exp(-((x-cx)/rx)**2 - ((z-cz)/rz)**2)


def ground(x, z):
    # Low valley floor and two near spurs remain below the lookout altitude.
    # Increasing angular elevation of successive crests keeps far ground visible.
    h = 78 + 11*noise(x+1200,z-400,1800) + 3*noise(x,z,460)
    for landform in [(-950,-1400,700,650,125), (1200,-1950,900,800,145)]:
        h += hill(x,z,*landform)
    for depth,width,base,peaks in [
        (4200,650,110,[(-1800,1600,230),(2300,1300,150)]),
        (7600,1100,330,[(-400,2100,320),(4100,1900,210)]),
        (13500,1800,660,[(-4700,3200,720),(4400,2900,580)]),
    ]:
        crest = base + sum(a*math.exp(-((x-c)/r)**2) for c,r,a in peaks)
        crest += .12*base*noise(x+depth,0,750)
        centre = -depth + 260*noise(x-depth,0,1600)
        section = math.exp(-((z-centre)/width)**2)
        h += section*(crest+14*noise(x,z,240))
    # The rotated Sun views also receive real distant land, with no finite-sheet gap.
    for landform in [(-7400,1200,1600,4600,390),(-13300,0,1900,6500,940),
                     (8200,-500,1700,4900,450),(14500,1600,2100,7200,1050),
                     (-6000,10500,3300,2200,670),(4200,13000,4500,2300,890)]:
        h += hill(x,z,*landform)
    # A local, flat-topped rock lookout. Its front drops into the valley rather
    # than making a bowl around the camera. No periodic/radial displacement.
    def smooth(t):
        t=max(0,min(1,t))
        return t*t*(3-2*t)
    shelf=(1-smooth((abs(x)-55)/100))*(1-smooth((-z-45)/130))*(1-smooth((z-60)/100))
    if x < -16:
        west = -x - 16
        descent = west*west/(west+24)
        half_width = 55 + 1.1*descent
        outer_width = 125 + 0.25*descent
        flank = 1-smooth((abs(z-16)-half_width)/outer_width)
        spur = math.exp(-descent/1700)*flank
        shelf = max(shelf, spur)
    local_detail=.25*noise(x,z,12)+.06*noise(x,z,2.6)
    return h*(1-shelf)+(240+local_detail)*shelf


def normals_from_faces(p, tris):
    n = [[0.,0.,0.] for _ in p]
    for a,b,c in tris:
        face = cross(sub(p[b],p[a]),sub(p[c],p[a]))
        for i in (a,b,c):
            for j in range(3):
                n[i][j] += face[j]
    return [unit(v) for v in n]


def heightfield_normals(positions):
    # Differentiate the resolved height samples, independent of triangle-star
    # parity. Area weighting on this unequal grid created alternating normals
    # and visible ribs at a grazing Sun. Do not add analytic detail absent from
    # the represented mesh.
    def derivative(k, step, axis, start, end):
        if start:
            return ((positions[k+step][1]-positions[k][1]) /
                    (positions[k+step][axis]-positions[k][axis]))
        if end:
            return ((positions[k][1]-positions[k-step][1]) /
                    (positions[k][axis]-positions[k-step][axis]))
        left = positions[k][axis]-positions[k-step][axis]
        right = positions[k+step][axis]-positions[k][axis]
        before = (positions[k][1]-positions[k-step][1])/left
        after = (positions[k+step][1]-positions[k][1])/right
        return (right*before+left*after)/(left+right)

    return [unit((-derivative(k, 1, 0, k%NX == 0, k%NX == NX-1),
                  1., -derivative(k, NX, 2, k//NX == 0, k//NX == NZ-1)))
            for k in range(len(positions))]


def terrain():
    # Unequal grid spacing concentrates vertices around the viewed foreground.
    xs = [math.copysign(18000*(abs(i-80)/80)**2.2,i-80) for i in range(NX)]
    zs = [-math.copysign(18000*(abs(i-92)/92)**2.4,i-92) for i in range(NZ)]
    # Match the actual glTF positions before constructing their derivatives.
    p = [struct.unpack('<3f', struct.pack('<3f', x, ground(x,z), z))
         for z in zs for x in xs]
    tris = []
    for j in range(NZ-1):
        for i in range(NX-1):
            a = j*NX+i
            # Positive Y winding. Alternate diagonals without adding geometry.
            if (i+j)%2:
                tris.extend([(a,a+1,a+NX),(a+1,a+NX+1,a+NX)])
            else:
                tris.extend([(a,a+1,a+NX+1),(a,a+NX+1,a+NX)])
    uv = [ground_uv(x, z) for z in zs for x in xs]
    return p, heightfield_normals(p), uv, tris


def ground_uv(x, z):
    """World-space repeat, enlarged smoothly with distance from the camera so far ground keeps detail."""
    dx, dz = x-GROUND_TILE_CENTRE[0], z-GROUND_TILE_CENTRE[1]
    scale = (1+(dx*dx+dz*dz)/GROUND_TILE_GROWTH_RADIUS**2)**(-GROUND_TILE_GROWTH/2)/GROUND_TILE_METRES
    return dx*scale, -dz*scale


def surface_height(mesh,x,z):
    # Exact interpolation of the float32 source triangles, including diagonals.
    p=mesh[0]
    xs=[p[i][0] for i in range(NX)]
    depths=[-p[j*NX][2] for j in range(NZ)]
    i=max(0,min(NX-2,bisect.bisect_right(xs,x)-1))
    j=max(0,min(NZ-2,bisect.bisect_right(depths,-z)-1))
    a=j*NX+i
    tris=[(a,a+1,a+NX),(a+1,a+NX+1,a+NX)] if (i+j)%2 else [(a,a+1,a+NX+1),(a,a+NX+1,a+NX)]
    for tri in tris:
        a,b,c=[p[k] for k in tri]
        denominator=(b[2]-c[2])*(a[0]-c[0])+(c[0]-b[0])*(a[2]-c[2])
        u=((b[2]-c[2])*(x-c[0])+(c[0]-b[0])*(z-c[2]))/denominator
        v=((c[2]-a[2])*(x-c[0])+(a[0]-c[0])*(z-c[2]))/denominator
        if min(u,v,1-u-v)>=-1e-6:
            return u*a[1]+v*b[1]+(1-u-v)*c[1]
    raise AssertionError('point outside selected terrain cell')


def round_mesh(segments, rings, rock=False):
    p, uv = [(0.,1.,0.)], [(0.5,0.)]
    for j in range(1,rings):
        theta = math.pi*j/rings
        for i in range(segments):
            phi = math.tau*i/segments
            v = (math.sin(theta)*math.cos(phi),math.cos(theta),math.sin(theta)*math.sin(phi))
            if rock:
                r = 1 + .13*noise(v[0]*3+11,v[2]*3+v[1]*2,1) + .07*hash2(i,j)
                v = (v[0]*r*1.35,v[1]*r*.82,v[2]*r)
            p.append(v)
            uv.append((i/segments,j/rings))
    bottom = len(p)
    p.append((0.,-.82 if rock else -1.,0.))
    uv.append((.5,1.))
    tris = []
    for i in range(segments):
        k=(i+1)%segments
        tris.append((0,1+k,1+i))
        for j in range(rings-2):
            a=1+j*segments+i
            b=1+j*segments+k
            tris.extend([(a,b,a+segments),(b,b+segments,a+segments)])
        tris.append((bottom,1+(rings-2)*segments+i,1+(rings-2)*segments+k))
    n = normals_from_faces(p,tris) if rock else list(p)
    return p,n,uv,tris


def validate(p,n,uv,tris):
    assert len(p)==len(n)==len(uv)
    assert all(math.isfinite(v) for row in p+n+uv for v in row)
    assert all(abs(dot(v,v)-1)<1e-5 for v in n)
    smallest, agreement = float('inf'), 1.
    for tri in tris:
        assert len(set(tri)) == 3 and min(tri)>=0 and max(tri)<len(p)
        a,b,c=tri
        face=cross(sub(p[b],p[a]),sub(p[c],p[a]))
        area=math.sqrt(dot(face,face))/2
        assert area > 1e-8
        smallest=min(smallest,area)
        avg=unit(tuple(n[a][j]+n[b][j]+n[c][j] for j in range(3)))
        d=dot(unit(face),avg)
        assert d>0, (tri,d)
        agreement=min(agreement,d)
    return {"vertices":len(p),"triangles":len(tris),"minimum_triangle_area_m2":smallest,
            "minimum_face_normal_agreement":agreement,
            "bounds_m":{"min":[min(v[j] for v in p) for j in range(3)],
                        "max":[max(v[j] for v in p) for j in range(3)]}}


def write_mesh(name, mesh, material):
    p,n,uv,tris=mesh
    # Validate the actual serialized float32 representation, not just Python float64.
    f32=lambda rows:[struct.unpack('<'+'f'*len(v),struct.pack('<'+'f'*len(v),*v)) for v in rows]
    p,n,uv=f32(p),f32(n),f32(uv)
    report=validate(p,n,uv,tris)
    buffers,views,accessors=[],[],[]
    offset=0
    for rows,fmt,kind,component,target in [(p,'f','VEC3',5126,34962),(n,'f','VEC3',5126,34962),
                                        (uv,'f','VEC2',5126,34962), (tris,'H','SCALAR',5123,34963)]:
        values=[v for row in rows for v in row]
        data=struct.pack('<'+str(len(values))+fmt,*values)
        views.append({"buffer":0,"byteOffset":offset,"byteLength":len(data),"target":target})
        accessor={"bufferView":len(views)-1,"componentType":component,"count":len(values) if kind=='SCALAR' else len(rows),"type":kind}
        if len(accessors)==0:
            accessor.update(min=report['bounds_m']['min'],max=report['bounds_m']['max'])
        accessors.append(accessor)
        pad=b'\0'*((-len(data))%4)
        buffers.append(data+pad)
        offset+=len(data)+len(pad)
    blob=b''.join(buffers)
    doc={"asset":{"version":"2.0","generator":"Thrum authored dry-valley stdlib recipe v3"},
         "scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":name}],
         "meshes":[{"name":name,"primitives":[{"attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2},"indices":3,"material":0,"mode":4}]}],
         "materials":[{"name":name,"pbrMetallicRoughness":material}],
         "buffers":[{"uri":name+'.bin',"byteLength":len(blob)}],"bufferViews":views,"accessors":accessors}
    (ROOT/(name+'.bin')).write_bytes(blob)
    (ROOT/(name+'.gltf')).write_text(json.dumps(doc,separators=(',',':'))+'\n')
    report['sha256_bin']=hashlib.sha256(blob).hexdigest()
    return report


def sketch_png(surfaces):
    # Scanline composition sketch only, painter ordering; no renderer emulation.
    width,height=960,540
    pixels=bytearray(bytes([185,199,206])*(width*height))
    for depth,points,colour in sorted(surfaces,reverse=True):
        rgb=bytes.fromhex(colour[1:])
        first=max(0,math.ceil(min(y for x,y in points)-.5))
        last=min(height-1,math.floor(max(y for x,y in points)-.5))
        for row in range(first,last+1):
            y=row+.5
            cuts=[]
            for a,b in zip(points,points[1:]+points[:1]):
                if (a[1]<=y<b[1]) or (b[1]<=y<a[1]):
                    cuts.append(a[0]+(y-a[1])*(b[0]-a[0])/(b[1]-a[1]))
            if len(cuts)==2:
                x0=max(0,math.ceil(min(cuts)-.5))
                x1=min(width,math.ceil(max(cuts)-.5))
                if x1>x0:
                    start=(row*width+x0)*3
                    pixels[start:start+(x1-x0)*3]=rgb*(x1-x0)
    def chunk(kind,data):
        return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
    rows=b''.join(b'\0'+pixels[y*width*3:(y+1)*width*3] for y in range(height))
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,2,0,0,0))
    png+=chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b'')
    (EVIDENCE/'overview.png').write_bytes(png)


def overview(scene):
    # Geometric composition aid only: coarse SVG, no sky/atmosphere/engine emulation.
    eye=scene['camera']['position_m']
    forward=unit(sub(scene['camera']['target_m'],eye))
    right=unit(cross(forward,(0,1,0)))
    up=cross(right,forward)
    focal=270/math.tan(math.radians(scene['camera']['vertical_fov_degrees']/2))
    surfaces=[]
    for item in scene['placements']:
        name=Path(item['mesh']).stem
        p,n,uv,tris=terrain() if name=='terrain' else round_mesh(48,24) if name=='sphere' else round_mesh(16,8,True)
        if name=='terrain':
            # 4x4 cell downsample, solely for the small overview.
            tris=[]
            for j in range(0,NZ-1,2):
                for i in range(0,NX-1,2):
                    a=j*NX+i
                    tris.extend([(a,a+2,a+2+2*NX),(a,a+2+2*NX,a+2*NX)])
        p=[tuple(v[k]*item['scale'][k]+item['translation_m'][k] for k in range(3)) for v in p]
        for tri in tris:
            points=[p[i] for i in tri]
            rel=[sub(v,eye) for v in points]
            depths=[dot(v,forward) for v in rel]
            if min(depths)<.1:
                continue
            projected=[(480+focal*dot(v,right)/d,270-focal*dot(v,up)/d) for v,d in zip(rel,depths)]
            if max(v[0] for v in projected)<0 or min(v[0] for v in projected)>960 or max(v[1] for v in projected)<0 or min(v[1] for v in projected)>540:
                continue
            normal=unit(cross(sub(points[1],points[0]),sub(points[2],points[0])))
            if dot(normal,rel[0])>=0:
                continue
            light=.48+.52*max(0,dot(normal,unit((-.8,1,.3))))
            rgb=item['material']['baseColorFactor'][:3]
            colour='#'+''.join(f'{int(255*(v*light)**(1/2.2)):02x}' for v in rgb)
            surfaces.append((sum(depths)/3,projected,colour))
    svg=['<svg xmlns="http://www.w3.org/2000/svg" width="960" height="540" viewBox="0 0 960 540">', '<rect width="960" height="540" fill="#b9c7ce"/>']
    for depth,points,colour in sorted(surfaces,reverse=True):
        coords=' '.join(f'{x:.1f},{y:.1f}' for x,y in points)
        svg.append(f'<polygon points="{coords}" fill="{colour}" stroke="{colour}" stroke-width=".25"/>')
    svg.append('<rect width="960" height="25" fill="#18232c"/><text x="12" y="17" font-family="sans-serif" font-size="12" fill="white">Composition overview only · coarse geometry · no atmosphere or engine shading</text></svg>')
    (EVIDENCE/'overview.svg').write_text('\n'.join(svg)+'\n')
    sketch_png(surfaces)


def fft(values, inverse=False):
    """Iterative radix-2 complex FFT of a power-of-two list (unscaled)."""
    n = len(values)
    a = list(values)
    j = 0
    for i in range(1, n):
        bit = n >> 1
        while j & bit:
            j ^= bit
            bit >>= 1
        j |= bit
        if i < j:
            a[i], a[j] = a[j], a[i]
    sign = 1 if inverse else -1
    length = 2
    while length <= n:
        step = complex(math.cos(2*math.pi/length), sign*math.sin(2*math.pi/length))
        half = length//2
        twiddles = [1+0j]
        for _ in range(half-1):
            twiddles.append(twiddles[-1]*step)
        for start in range(0, n, length):
            for k in range(half):
                u = a[start+k]
                v = a[start+k+half]*twiddles[k]
                a[start+k] = u+v
                a[start+k+half] = u-v
        length <<= 1
    return a


def spectral_noise(size, amplitude, rng):
    """Unit-variance tileable isotropic noise: random phases under amplitude(k), k in cycles per tile."""
    half = size//2
    rows = []
    for y in range(size):
        ky = y if y <= half else y-size
        row = []
        for x in range(size):
            kx = x if x <= half else x-size
            a = amplitude(math.hypot(kx, ky))
            row.append(a*complex(rng.gauss(0, 1), rng.gauss(0, 1)) if a else 0j)
        rows.append(fft(row, True) if any(row) else row)
    columns = [fft([rows[y][x] for y in range(size)], True) for x in range(size)]
    flat = [columns[x][y].real for y in range(size) for x in range(size)]
    mean = sum(flat)/len(flat)
    std = (sum((v-mean)**2 for v in flat)/len(flat))**.5
    return [(v-mean)/std for v in flat]


def band(low, high, slope):
    """Amplitude k**-slope between low and high cycles per tile, with smooth half-octave shoulders."""
    def amplitude(k):
        if k < low/1.5 or k > high*1.5:
            return 0.
        a = k**-slope
        if k < low:
            t = (k-low/1.5)/(low-low/1.5); a *= t*t*(3-2*t)
        if k > high:
            t = (high*1.5-k)/(high*.5); a *= t*t*(3-2*t)
        return a
    return amplitude


def upsample(field, cells, size):
    """Periodic cubic B-spline resample of a cells x cells field to size x size."""
    taps = []
    for x in range(size):
        f = (x+.5)*cells/size - .5
        i = math.floor(f); t = f-i
        w = ((1-t)**3/6, (3*t**3-6*t*t+4)/6, (-3*t**3+3*t*t+3*t+1)/6, t**3/6)
        taps.append(([(i+k-1) % cells for k in range(4)], w))
    lattice = [field[y*cells:(y+1)*cells] for y in range(cells)]
    rows = [[sum(w[k]*row[c[k]] for k in range(4)) for c, w in taps] for row in lattice]
    out = []
    for c, w in taps:
        r0, r1, r2, r3 = (rows[k] for k in c)
        out.extend(w[0]*a+w[1]*b+w[2]*d+w[3]*e for a, b, d, e in zip(r0, r1, r2, r3))
    return out


def ground_albedo():
    """Tileable mineral ground: isotropic grain from about 2.7 m down to one texel, per-texel grit and gravel
    clasts with contact shade. Each channel's linear mean equals the old neutral, so the far ground's average
    colour and the scene's material factor are unchanged."""
    size = GROUND_SYNTH_SIZE
    neutral_code = 218
    neutral = ((neutral_code/255+.055)/1.055)**2.4
    rng = random.Random(20261107)
    n = size*size
    texel = GROUND_TILE_METRES/size
    # Isotropic, tileable spectra with nothing coarser than about 2.7 m (k >= 6 per tile), so no patch marks the repeat.
    texture = spectral_noise(size, band(16, 400, 1.05), rng)
    hue = upsample(spectral_noise(128, band(12, 40, .3), rng), 128, size)
    patch = upsample(spectral_noise(128, band(12, 24, 0), rng), 128, size)
    broad = upsample(spectral_noise(128, band(6, 16, 1.), rng), 128, size)
    grit = [rng.random()*2-1 for _ in range(n)]
    lum = [1 + .12*a + .06*b + .06*g for a, b, g in zip(texture, broad, grit)]
    red = [l*(1+.05*h) for l, h in zip(lum, hue)]
    green = [l*(1-.01*h) for l, h in zip(lum, hue)]
    blue = [l*(1-.07*h) for l, h in zip(lum, hue)]
    # Gravel: clasts 1.5 to 5 cm across, denser where the patch noise is high (pavement), with contact shade.
    palette = [(.50, .52, .62), (.62, .60, .64), (.82, .62, .55), (1.18, 1.16, 1.12), (1.08, .98, .86)]
    clasts = int(110*GROUND_TILE_METRES**2)
    for _ in range(clasts):
        cx, cy = rng.random()*size, rng.random()*size
        if rng.random() > .35 + .3*patch[int(cy)*size+int(cx)]:
            continue
        radius = (.75 + 1.85*rng.random()**2) * (.0156/texel)
        tint = palette[rng.randrange(len(palette))]
        shade = .9 + .2*rng.random()
        reach = radius + 1.5
        for y in range(math.floor(cy-reach), math.ceil(cy+reach)+1):
            for x in range(math.floor(cx-reach), math.ceil(cx+reach)+1):
                d = math.hypot(x+.5-cx, y+.5-cy)
                k = (y % size)*size + x % size
                if d < radius:
                    cover = min(1., radius-d+.5)
                    for channel, value in zip((red, green, blue), tint):
                        channel[k] += cover*(value*shade - channel[k])
                elif d < reach:
                    dim = 1 - .12*(reach-d)/1.5
                    red[k] *= dim; green[k] *= dim; blue[k] *= dim
    rows = bytearray()
    channels = []
    for channel in (red, green, blue):
        for _ in range(4):
            mean = sum(channel)/n
            channel = [min(1., max(0., v*neutral/mean)) for v in channel]
        channels.append(channel)
    factor = size//GROUND_TEXTURE_SIZE
    size = GROUND_TEXTURE_SIZE
    channels = [[sum(channel[(y*factor+j)*size*factor + x*factor+i] for j in range(factor) for i in range(factor))/factor**2
                 for y in range(size) for x in range(size)] for channel in channels]
    for y in range(size):
        rows.append(0)
        for x in range(size):
            k = y*size+x
            for channel in channels:
                rows.append(round(255*(1.055*channel[k]**(1/2.4)-.055)))
    return size, rows, neutral


def png(width, height, colour_type, rows, up=False):
    """PNG from filter-0 rows; up=True re-filters each row against the one above (smaller for noisy images)."""
    def chunk(kind, data):
        return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
    if up:
        stride = len(rows)//height
        lines = [rows[y*stride+1:(y+1)*stride] for y in range(height)]
        rows = b''.join(b'\x02'+bytes((a-b)&255 for a, b in zip(line, previous))
                        for line, previous in zip(lines, [bytes(stride-1)]+lines[:-1]))
    header = struct.pack('>IIBBBBB',width,height,8,colour_type,0,0,0)
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',header)+chunk(b'IDAT',zlib.compress(rows,9 if up else -1))+chunk(b'IEND',b'')


def main():
    global ROOT, EVIDENCE
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir',type=Path,default=ROOT)
    parser.add_argument('--evidence-dir',type=Path,default=EVIDENCE)
    options=parser.parse_args()
    ROOT,EVIDENCE=options.output_dir.resolve(),options.evidence_dir.resolve()
    ROOT.mkdir(parents=True,exist_ok=True)
    EVIDENCE.mkdir(parents=True,exist_ok=True)
    materials={"terrain":{"baseColorFactor":[.28,.23,.17,1],"metallicFactor":0,"roughnessFactor":.92},
               "sphere":{"baseColorFactor":[.72,.74,.76,1],"metallicFactor":1,"roughnessFactor":.2},
               "rock":{"baseColorFactor":[.34,.29,.23,1],"metallicFactor":0,"roughnessFactor":.85}}
    report={"python":platform.python_version(),"meshes":{}}
    meshes={'terrain':terrain(),'sphere':round_mesh(48,24),'rock':round_mesh(16,8,True)}
    for name,mesh in meshes.items():
        report['meshes'][name]=write_mesh(name,mesh,materials[name])
    terrain_positions=[struct.unpack('<3f',struct.pack('<3f',*v)) for v in meshes['terrain'][0]]
    source_terrain=(terrain_positions,*meshes['terrain'][1:])
    sphere_x,sphere_z=-3.4,-5.
    rock_x,rock_z=3.8,-7.
    size, albedo, neutral = ground_albedo()
    ground_material = {**materials['terrain'],
                       "baseColorFactor":[*(value/neutral for value in materials['terrain']['baseColorFactor'][:3]),1]}
    placements=[{"mesh":"terrain.gltf","texture":"ground","translation_m":[0,0,0],"scale":[1,1,1],"material":ground_material},
                {"mesh":"sphere.gltf","translation_m":[sphere_x,surface_height(source_terrain,sphere_x,sphere_z)+1.5-.025,sphere_z],"scale":[1.5,1.5,1.5],"material":materials['sphere']},
                {"mesh":"rock.gltf","translation_m":[rock_x,surface_height(source_terrain,rock_x,rock_z)-min(v[1] for v in meshes['rock'][0])-.12,rock_z],"scale":[1,1,1],"material":materials['rock']}]
    report['minimum_subject_vertex_clearance_m']={}
    for item in placements[1:]:
        name=Path(item['mesh']).stem
        world=[tuple(v[k]*item['scale'][k]+item['translation_m'][k] for k in range(3)) for v in meshes[name][0]]
        clearance=min(y-surface_height(source_terrain,x,z) for x,y,z in world)
        report['minimum_subject_vertex_clearance_m'][name]=clearance
        assert -.16<clearance<0, (name,clearance)
    camera={"position_m":[0,ground(0,16)+5.,16],"target_m":[0,ground(0,16)+1.,-60],"up":[0,1,0],
            "vertical_fov_degrees":46.,"aspect":16/9,"near_m":.1,"far_m":30000.,"fixed_for_all_sun_times":True}
    scene={"id":"g3s-authored-dry-valley-v1","convention":"metres, right handed, Y up, view toward -Z; solar sunrise/sunset +/-X", "camera":camera,"placements":placements,
           "atmosphere":{"camera_altitude_m":camera['position_m'][1],"ground_plane_altitude_m":0,"distance_scale":1,"note":"Use actual world altitude and metre distances; no fog gain."},
           "reference_scope":"The four Kiara photos were visually inspected, used only for composition and material judgement; no photographic runtime asset."}
    # Only values consumed by the game belong in the embedded runtime recipe.
    recipe={"camera":{k:camera[k] for k in ["position_m","target_m","vertical_fov_degrees","near_m","far_m"]},
            "placements":placements}
    (ROOT/'scene.json').write_text(json.dumps(recipe,indent=2)+'\n')
    # One neutral compression block leaves per-placement linear PBR factors unchanged.
    (ROOT/'white.png').write_bytes(png(4,4,6,(b'\0'+b'\xff'*16)*4))
    (ROOT/'ground.png').write_bytes(png(size,size,2,albedo,up=True))
    overview(scene)
    report['total_triangles']=sum(v['triangles'] for v in report['meshes'].values())
    assert report['total_triangles']<=64000
    report['recipe_sha256']=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    report['source_outputs_bytes']=sum(p.stat().st_size for p in ROOT.iterdir() if p.is_file() and p.name!='validation.json')
    assert report['source_outputs_bytes']<16*1024*1024
    (EVIDENCE/'validation.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    main()
