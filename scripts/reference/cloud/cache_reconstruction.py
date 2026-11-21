"""Reproduce Sun-cache interpolation alternatives in independent f64 arithmetic.

Inputs are the retained Metal stage snapshots and directly marched sample table.
This emulates the cache producer at its 40 m integration spacing, quantizes its
nodes to fp16, then compares linear, Catmull-Rom and shape-preserving interpolation.
It calls no production cloud kernel and does not load the cached Sun textures.
"""
import sys,csv,math,struct,functools,time,argparse
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from cloud_lighting_reference import Field,floats,source
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--input',type=Path,required=True)
parser.add_argument('--samples',type=Path,default=Path('tests/fixtures/clouds/lighting-diagnosis-samples.csv'))
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
ROOT=args.input
rows=list(csv.DictReader(args.samples.open()))

def cubic(p,t):
    return p[1]+0.5*t*(p[2]-p[0]+t*(2*p[0]-5*p[1]+4*p[2]-p[3]+t*(3*(p[1]-p[2])+p[3]-p[0])))

def pchip(p,t):
    d=[p[i+1]-p[i] for i in range(3)]
    slope=lambda a,b:2*a*b/(a+b) if a*b>0 else 0
    m0,m1=slope(d[0],d[1]),slope(d[1],d[2])
    return p[1]*(2*t**3-3*t*t+1)+m0*(t**3-2*t*t+t)+p[2]*(-2*t**3+3*t*t)+m1*(t**3-t*t)

results=[]
for case in sorted({r['case'] for r in rows}):
    data=floats(ROOT/case/'buffer-0.bin');field=Field(ROOT/case)
    camera=list(data[:3]);sun=list(data[8:11]);top=data[30];base=data[29]
    origin=[math.floor(camera[a]/5120)*5120-10240 for a in [0,2]]
    dh=(top-base)/47;steps=math.ceil(dh/(sun[1]*40));sub=dh/steps
    @functools.cache
    def column(x,z):
        xy=[origin[0]+(x+.5)*80,origin[1]+(z+.5)*80]
        tau=0;values=[0]
        for k in range(1,48):
            for j in range(steps):
                height=top-((k-1)*dh+(j+.5)*sub)
                p=[xy[0]+sun[0]*(height-base)/sun[1],height,xy[1]+sun[2]*(height-base)/sun[1]]
                tau+=field.density(p,0,coarse=True)*sub/sun[1]
            values.append(struct.unpack('<e',struct.pack('<e',min(65504,tau)))[0])
        return values
    start=time.monotonic();total=[0.,0.,0.];replay=0.;transmission=1.
    for r in (r for r in rows if r['case']==case):
        sample=int(r['sample']);at=32+sample*32
        point=list(data[at:at+3]);distance=data[at+3]
        reach=min(120,(top-point[1])/sun[1]);q=[p+s*reach for p,s in zip(point,sun)]
        projection=[q[a]-sun[a]*(q[1]-base)/sun[1] for a in [0,2]]
        coord=[(p-o)/80-.5 for p,o in zip(projection,origin)]+[(top-q[1])/(top-base)*47]
        lo=[math.floor(v) for v in coord];frac=[v-i for v,i in zip(coord,lo)]
        value=lambda x,z,y:column(lo[0]+x,lo[1]+z)[min(47,max(0,lo[2]+y))]
        linear=sum(value(x,z,y)*(frac[0] if x else 1-frac[0])*(frac[1] if z else 1-frac[1])*(frac[2] if y else 1-frac[2]) for x in range(2) for z in range(2) for y in range(2))
        mix=lambda p,t:p[1]*(1-t)+p[2]*t
        variants=[linear]+[fy([fx([fx([value(x,z,y) for x in range(-1,3)],frac[0]) for z in range(-1,3)],frac[1]) for y in range(-1,3)],frac[2]) for fx,fy in [(cubic,cubic),(pchip,pchip),(pchip,mix),(mix,pchip)]]
        local=sum(field.density([p+s*(j+.5)*reach/3 for p,s in zip(point,sun)],distance/129)*reach/3 for j in range(3))
        row=dict(case=case,sample=sample,actual_T=float(r['sun_T_production']),reference_T=float(r['sun_T_split_reference']))
        sigma=data[at+4];step=data[at+6];removed=-math.expm1(-sigma*step)
        for i,(name,tau) in enumerate(zip(['linear','cubic','pchip','spatial','height'],variants)):
            row[name+'_T']=math.exp(-local-max(0,tau))
            solar=list(data[at+12:at+15]);phase=list(data[24:28]);vertical=list(data[at+9:at+11]);mean=data[11];hemis=[list(data[16:19]),list(data[20:23])]
            light,_=source(solar,phase,local+max(0,tau),vertical,mean,hemis)
            if i<3:total[i]+=transmission*removed*(data[at+25]*light[1]+data[at+29])
        transmission*=math.exp(-sigma*step)
        results.append(row)
    print(case,'seconds',time.monotonic()-start,'columns',column.cache_info().currsize,'S',total,flush=True)
    for name in ['linear','cubic','pchip','spatial','height']:
        vs=[r for r in results if r['case']==case];errors=[abs(v[name+'_T']-v['reference_T']) for v in vs]
        print(name,'max',max(errors),'rms',(sum(e*e for e in errors)/len(errors))**.5,flush=True)
args.output.parent.mkdir(parents=True,exist_ok=True)
with args.output.open('w') as f:
    writer=csv.DictWriter(f,fieldnames=results[0].keys(),lineterminator='\n');writer.writeheader();writer.writerows(results)
