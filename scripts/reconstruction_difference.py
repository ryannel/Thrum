#!/usr/bin/env python3
"""Whole-frame D65 CIELAB/CIEDE2000, without masks or spatial reduction.

Equations: Sharma, Wu and Dalal (2005), doi:10.1002/col.20070.
The published supplementary pairs below check the implementation independently.
https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/
"""
import math
from functools import lru_cache


@lru_cache(maxsize=65536)
def lab(pixel):
    def decode(c):
        c /= 255
        return c / 12.92 if c <= .04045 else ((c + .055) / 1.055) ** 2.4
    r, g, b = map(decode, pixel)
    xyz = ((.4124564*r+.3575761*g+.1804375*b)/.95047,
           .2126729*r+.7151522*g+.0721750*b,
           (.0193339*r+.1191920*g+.9503041*b)/1.08883)
    def f(t):
        return t ** (1/3) if t > (6/29)**3 else t/(3*(6/29)**2)+4/29
    x, y, z = map(f, xyz)
    return 116*y-16, 500*(x-y), 200*(y-z)


def delta_e(first, second):
    l1, a1, b1 = first
    l2, a2, b2 = second
    c1, c2 = math.hypot(a1,b1), math.hypot(a2,b2)
    c = (c1+c2)/2
    g = .5*(1-math.sqrt(c**7/(c**7+25**7)))
    ap1, ap2 = (1+g)*a1, (1+g)*a2
    cp1, cp2 = math.hypot(ap1,b1), math.hypot(ap2,b2)
    def hue(a,b):
        return math.degrees(math.atan2(b,a)) % 360 if a or b else 0
    h1, h2 = hue(ap1,b1), hue(ap2,b2)
    dl, dc = l2-l1, cp2-cp1
    dh = h2-h1
    if cp1*cp2 == 0:
        dh = 0
    elif dh > 180:
        dh -= 360
    elif dh < -180:
        dh += 360
    dh = 2*math.sqrt(cp1*cp2)*math.sin(math.radians(dh/2))
    lm, cm = (l1+l2)/2, (cp1+cp2)/2
    if cp1*cp2 == 0:
        hm = h1+h2
    elif abs(h1-h2) <= 180:
        hm = (h1+h2)/2
    elif h1+h2 < 360:
        hm = (h1+h2+360)/2
    else:
        hm = (h1+h2-360)/2
    def cos(deg):
        return math.cos(math.radians(deg))
    t = 1-.17*cos(hm-30)+.24*cos(2*hm)+.32*cos(3*hm+6)-.20*cos(4*hm-63)
    sl = 1+.015*(lm-50)**2/math.sqrt(20+(lm-50)**2)
    sc, sh = 1+.045*cm, 1+.015*cm*t
    rt = -2*math.sqrt(cm**7/(cm**7+25**7))*math.sin(math.radians(60*math.exp(-((hm-275)/25)**2)))
    x, y, z = dl/sl, dc/sc, dh/sh
    return math.sqrt(max(0,x*x+y*y+z*z+rt*y*z))


def compare(first, second):
    assert len(first) == len(second) and len(first) % 3 == 0
    values = sorted(delta_e(lab(tuple(first[i:i+3])),lab(tuple(second[i:i+3])))
                    for i in range(0,len(first),3))
    return {'p99': values[math.ceil(.99*len(values))-1], 'maximum': values[-1], 'pixels': len(values)}


def self_check():
    for first, second, expected in [
        ((50,2.6772,-79.7751),(50,0,-82.7485),2.0425),
        ((50,3.1571,-77.2803),(50,0,-82.7485),2.8615),
        ((50,2.8361,-74.0200),(50,0,-82.7485),3.4412),
        ((50,-1.3802,-84.2814),(50,0,-82.7485),1.0000),
        ((50,0,0),(50,-1,2),2.3669),
        ((50,2.49,-.001),(50,-2.49,.0009),7.1792),
        ((50,2.49,-.001),(50,-2.49,.001),7.1792),
        ((50,2.49,-.001),(50,-2.49,.0011),7.2195),
        ((50,-.001,2.49),(50,.0009,-2.49),4.8045),
        ((50,-.001,2.49),(50,.001,-2.49),4.8045),
        ((50,-.001,2.49),(50,.0011,-2.49),4.7461),
        ((60.2574,-34.0099,36.2677),(60.4626,-34.1751,39.4387),1.2644),
    ]:
        assert abs(delta_e(first,second)-expected) < .0001


if __name__ == '__main__':
    self_check()
    print('CIEDE2000 published-pair checks passed')
