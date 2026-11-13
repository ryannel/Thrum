"""Independent 70-digit Decimal Gerstner oracle; stdout is extended.txt.

Input conversion is exact from binary64, to test arithmetic rather than decimal
parameter rounding. Taylor sin/cos and Newton square root are Decimal operations.
No production evaluator, FFT, third-party library or Broadsides source is used.
"""
from decimal import Decimal as D, getcontext
getcontext().prec = 70
PI = D('3.141592653589793238462643383279502884197169399375105820974944592307816406')
TAU = 2 * PI

def sc(x):
    x %= TAU
    sine, cosine, s, c = x, D(1), x, D(1)
    for n in range(1, 160):
        s *= -x*x / ((2*n)*(2*n+1))
        c *= -x*x / ((2*n-1)*(2*n))
        sine += s
        cosine += c
    return sine, cosine

components = [[D(v) for v in row] for row in [(0.17, 0.08, 1.4, 0.7), (-0.04, 0.11, 0.8, 2.1)]]
point = [D(20000), D(-17000)]
t = D(86400)
u = point[:]
for _ in range(300):
    following = point[:]
    for kx,kz,a,p in components:
        k = (kx*kx+kz*kz).sqrt()
        w = (D(9.80665)*k).sqrt()
        s,c = sc(kx*u[0]+kz*u[1]-w*t+p)
        following[0] += a*kx/k*s
        following[1] += a*kz/k*s
    u = following
h = D(0); grad = [D(0),D(0)]; vel = [D(0)]*3
j = [[D(1),D(0)],[D(0),D(1)]]
for kx,kz,a,p in components:
    k = (kx*kx+kz*kz).sqrt(); w = (D(9.80665)*k).sqrt()
    s,c = sc(kx*u[0]+kz*u[1]-w*t+p)
    h += a*c
    grad[0] -= a*kx*s; grad[1] -= a*kz*s
    vel[0] += a*kx/k*w*c; vel[1] += a*w*s; vel[2] += a*kz/k*w*c
    for axis,v in enumerate([kx,kz]):
        for other,b in enumerate([kx,kz]):
            j[axis][other] -= a*v*b/k*c
det = j[0][0]*j[1][1]-j[0][1]*j[1][0]
slope = [(grad[0]*j[1][1]-grad[1]*j[1][0])/det, (grad[1]*j[0][0]-grad[0]*j[0][1])/det]
for value in [h,*slope,*vel,vel[1]-slope[0]*vel[0]-slope[1]*vel[2]]:
    print(value)
