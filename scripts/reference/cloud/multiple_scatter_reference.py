"""Independent nonabsorbing plane-parallel droplet transport; no render kernels.

The medium has unit single-scattering albedo, the approved 24 micrometre HG/Draine
phase, and a point Sun. Free paths are exponential in optical distance. Angular
sampling uses inverse-CDF HG and rejection sampling of the Draine factor.
The four-order comparator is integrated analytically through the slab and then
numerically over the outgoing hemisphere. No atmosphere or cloud cache is read.
"""
import math,random,time,json,argparse
from pathlib import Path
PI=math.pi
d=24
gh=math.exp(-.0990567/(d-1.67154));gd=math.exp(-2.20679/(d+3.91029)-.428934)
alpha=math.exp(3.62489-8.29288/(d+5.52825));w=math.exp(-.599085/(d-.641583)-.665888)
def hg(c,g):return (1-g*g)/(4*PI*(1+g*g-2*g*c)**1.5)
def phase(c):return (1-w)*hg(c,gh)+w*hg(c,gd)*(1+alpha*c*c)/(1+alpha*(1+2*gd*gd)/3)
def model(mu0,tau,nmu=128,nphi=256,first_only=False):
    result=0
    for i in range(nmu):
        mu=(i+.5)/nmu
        for j in range(nphi):
            c=-mu*mu0+math.sqrt((1-mu*mu)*(1-mu0*mu0))*math.cos((j+.5)*2*PI/nphi)
            phases=[phase(c)]+[hg(c,g) for g in [.43,.215,.1075]]
            orders=[(1,1,phases[0])] if first_only else zip([.7,.2,.07,.03],[1,.5,.25,.125],phases)
            for weight,b,p in orders:
                extinction=1/mu+b/mu0
                result+=weight*p*(-math.expm1(-tau*extinction))/extinction/mu0*2*PI/(nmu*nphi)
    return result
def monte_carlo(mu0,tau,count=50000,seed=1939,first_only=False):
    rng=random.Random(seed)
    def hg_cos(g):
        u=rng.random();s=(1-g*g)/(1-g+2*g*u)
        return max(-1,min(1,(1+g*g-s*s)/(2*g)))
    def scatter_cos():
        if rng.random()>=w:return hg_cos(gh)
        while True:
            c=hg_cos(gd)
            if rng.random()*(1+alpha)<=1+alpha*c*c:return c
    escaped=[0,0,0];events=0;mean=0;draws=0
    for _ in range(count):
        z=0;mu=-mu0
        # Depth z increases downwards; direction mu is outward (towards sky).
        for bounce in range(20000):
            z-=mu*(-math.log(max(1e-30,rng.random())))
            if z<=0:escaped[0]+=1;break
            if z>=tau:escaped[1]+=1;break
            if first_only and bounce==1:escaped[2]+=1;break
            c=scatter_cos();mean+=c;draws+=1
            mu=mu*c+math.sqrt(max(0,(1-mu*mu)*(1-c*c)))*math.cos(2*PI*rng.random())
            mu=max(-1,min(1,mu));events+=1
        else:escaped[2]+=1
    reflected=escaped[0]/count
    return {'photons':count,'reflected':reflected,'transmitted':escaped[1]/count,'unresolved':escaped[2],
            'reflection_standard_error':math.sqrt(reflected*(1-reflected)/count),'events':events,'sampled_phase_mean':mean/draws}
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--photons',type=int,default=50000)
parser.add_argument('--seed',type=int,default=1939)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
out=[]
for elevation in [45,60,10]:
    for tau in [16,64]:
        started=time.monotonic();mu=math.sin(math.radians(elevation));r=monte_carlo(mu,tau,args.photons,args.seed)
        r.update(elevation=elevation,optical_depth=tau,four_order_reflectance=model(mu,tau),four_order_coarse_quadrature=model(mu,tau,64,128),seconds=time.monotonic()-started,seed=args.seed)
        assert r['unresolved']==0, 'every photon must exit; unresolved paths cannot be discarded'
        assert abs(r['reflected']+r['transmitted']-1)<1e-12, 'passive photon balance'
        out.append(r);print(json.dumps(r),flush=True)
args.output.parent.mkdir(parents=True,exist_ok=True)
mu=math.sin(PI/3)
validation=monte_carlo(mu,2,args.photons,args.seed,first_only=True)
validation['analytic_single_scatter_reflection']=model(mu,2,first_only=True)
assert abs(validation['reflected']-validation['analytic_single_scatter_reflection'])<=5*validation['reflection_standard_error'], 'single-event transport must match its independent integral'
analytic_mean=(1-w)*gh+w*gd*(1+alpha*(3+2*gd*gd)/5)/(1+alpha*(1+2*gd*gd)/3)
result={'point_sun':True,'single_scattering_albedo':1,'droplet_diameter_micrometres':d,'analytic_phase_mean':analytic_mean,'single_scatter_validation':validation,'runs':out}
args.output.write_text(json.dumps(result,indent=2)+'\n')
print('single-scatter validation',json.dumps(validation),flush=True)
