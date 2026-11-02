import { floatToHalf, halfToFloat } from './reference.js';
import { analyzeDensity } from './morphology.js';
// Requested GPU verification only; none of these readbacks run in ordinary frames.
export async function runCoreReference(scene) {
  const g=scene.gpu,d=g.device;
  const params=new Float32Array(44);
  params.set([0,0,0,.0045],12);params.set([1500,1450,1500,.0018],16);
  params.set([0,.25,0,0],20);params.set([.85,.08,1,1],24);params.set([1e6,1,2,1],28);
  const uniform=g.buffer('core oracle uniform',176,GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST);
  d.queue.writeBuffer(uniform,0,params);
  const heads=g.buffer('core oracle head',48*64,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST);
  const bands=g.buffer('core oracle bands',32*49*4,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST);
  const hp=new Float32Array(48*16),bp=new Uint32Array(32*49);
  hp.set([0,5000,0,1500,24,900,1,28,1,1,0,1,28,.0015,.005,1]);
  for(let i=0;i<32;i++)bp[i*49]=1;
  d.queue.writeBuffer(heads,0,hp);d.queue.writeBuffer(bands,0,bp);
  const result=g.buffer('core oracle result',64,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC);
  const read=g.buffer('core oracle read',64,GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST);
  const code=scene.codes.source.replace('fn main(','fn sourceStep(')+`
  @group(0) @binding(15) var<storage,read_write> r:array<vec4f>;
  fn internalAt(x:vec3f)->vec3f{let shear=clamp((x.y-1100.)/8000.,0.,1.);return flow(x).xyz-vec3f(0,0,1.1+3.*shear);}
  @compute @workgroup_size(1) fn main(){let x=vec3f(0,5000,0);r[0]=vec4f(internalAt(x),0);
    r[1]=vec4f((internalAt(x+vec3f(1,0,0))-internalAt(x-vec3f(1,0,0)))*.5,0);
    r[2]=vec4f((internalAt(x+vec3f(0,1,0))-internalAt(x-vec3f(0,1,0)))*.5,0);
    r[3]=vec4f((internalAt(x+vec3f(0,0,1))-internalAt(x-vec3f(0,0,1)))*.5,0);}`;
  const pipeline=await g.compute('assembled core affine oracle',code);
  let e=d.createCommandEncoder();
  scene.compute(e,pipeline,g.group(pipeline,[[0,uniform],[9,scene.curl.createView()],[10,scene.repeat],[13,heads],[14,bands],[15,result]]),[1,1,1]);
  e.copyBufferToBuffer(result,0,read,0,64);g.submit(e);await read.mapAsync(GPUMapMode.READ);
  const actual=Array.from(new Float32Array(read.getMappedRange().slice(0)));read.unmap();
  const expected=[0,28,0,0,.0015,0,0,0,0,.005,0,0,0,0,.0015,0];
  const affineError=Math.max(...actual.map((v,i)=>Math.abs(v-expected[i])));
  const a=g.texture('mix oracle a',[8,8,8]),b=g.texture('mix oracle b',[8,8,8]);
  const mix=await g.compute('independent positive mix oracle',scene.codes.mix);
  const groups=[g.group(mix,[[0,uniform],[1,a.createView()],[2,b.createView()]]),g.group(mix,[[0,uniform],[1,b.createView()],[2,a.createView()]])];
  const mapped=g.buffer('mix oracle read',16384,GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST);
  const N=512,input=new Float64Array(N*4);
  for(let z=0;z<8;z++)for(let y=0;y<8;y++)for(let x=0;x<8;x++){
    const i=(z*8+y)*8+x,k=(x+2*y+3*z)%5;
    input.set([k<2?.5:0,k>2?.25:0,k<2?10:k>2?100:0,k===2?0:.01],i*4);
  }
  function upload(values){const raw=Uint16Array.from(values,floatToHalf);d.queue.writeTexture({texture:a},raw,{bytesPerRow:64,rowsPerImage:8},[8,8,8]);return Float64Array.from(raw,halfToFloat);}
  async function download(texture,encoder){encoder.copyTextureToBuffer({texture},{buffer:mapped,bytesPerRow:256,rowsPerImage:8},[8,8,8]);g.submit(encoder);await mapped.mapAsync(GPUMapMode.READ);const raw=new Uint16Array(mapped.getMappedRange().slice(0));mapped.unmap();const q=new Float64Array(N*4);for(let z=0;z<8;z++)for(let y=0;y<8;y++)for(let x=0;x<8;x++)for(let c=0;c<4;c++)q[((z*8+y)*8+x)*4+c]=halfToFloat(raw[(z*8+y)*128+x*4+c]);return q;}
  const start=upload(input);e=d.createCommandEncoder();scene.compute(e,mix,groups[0],[8,8,8]);const once=await download(b,e);
  const alpha=[1000*.25/187.5**2,1000*.25/181.25**2,1000*.25/187.5**2],central=1-2*alpha.reduce((s,v)=>s+v,0);
  let referenceError=0,birthRoundingBudget=0;
  // WGSL 15.7.6 allows either adjacent binary16 value, not only round-to-nearest.
  // Bound each store by one local half ULP plus a conservative f32 stencil budget.
  function halfUlp(v){return Math.max(2**-24,2**(Math.floor(Math.log2(Math.abs(v)||2**-14))-10));}
  for(let z=0;z<8;z++)for(let y=0;y<8;y++)for(let x=0;x<8;x++){
    const terms=[[x,y,z,central]];
    for(let axis=0;axis<3;axis++)for(const sign of [-1,1]){const p=[x,y,z];p[axis]=Math.max(0,Math.min(7,p[axis]+sign));terms.push([...p,alpha[axis]]);}
    const m=[0,0,0,0];for(const [xx,yy,zz,w] of terms){const i=((zz*8+yy)*8+xx)*4;const q=start[i]+start[i+1];m[0]+=w*start[i];m[1]+=w*start[i+1];m[2]+=w*q*start[i+2];m[3]+=w*start[i+3];}
    birthRoundingBudget+=halfUlp(m[3])+16*2**-23*.01;
    const ref=[m[0],m[1],m[0]+m[1]>1e-10?m[2]/(m[0]+m[1]):0,m[3]],o=((z*8+y)*8+x)*4;
    for(let c=0;c<4;c++)referenceError=Math.max(referenceError,Math.abs(once[o+c]-ref[c])/[.0005,.0005,.125,.00005][c]);
  }
  function sums(q){const s=[0,0,0,0];for(let i=0;i<q.length;i+=4){s[0]+=q[i];s[1]+=q[i+1];s[2]+=(q[i]+q[i+1])*q[i+2];s[3]+=q[i+3];}return s;}
  const before=sums(start),after=sums(once);
  const massError=Math.max(Math.abs(before[0]-after[0]),Math.abs(before[1]-after[1]));
  const birthError=Math.abs(before[3]-after[3]);
  const ageRelativeError=Math.abs(before[2]-after[2])/before[2];
  const merge=new Float64Array(N*4);for(let i=0;i<N;i++)if(i%8<2||i%8>=6)merge[i*4]=.5;
  upload(merge);e=d.createCommandEncoder();for(let step=0;step<160;step++)scene.compute(e,mix,groups[step%2],[8,8,8]);const merged=await download(a,e);
  const values=new Float32Array(N);for(let i=0;i<N;i++)values[i]=merged[i*4];
  const components=analyzeDensity(values,[8,8,8],[[0,0,0],[1500,1450,1500]]).substantialComponents;
  // Execute the actual joint carrier producer with distinguishable constant old charts.
  // Interior cells must retain the non-reset chart and spatially seed only the reset chart.
  const ca=g.texture('carrier oracle old A',[8,8,8]),cb=g.texture('carrier oracle old B',[8,8,8]);
  for(const [t,v] of [[ca,2],[cb,3]]){const raw=Uint16Array.from({length:N*4},(_,i)=>floatToHalf(i%4===3?1:v));d.queue.writeTexture({texture:t},raw,{bytesPerRow:64,rowsPerImage:8},[8,8,8]);}
  const carriers=await g.compute('actual joint carrier reset oracle',scene.codes.source.replace('fn main(','fn sourceStep(')+scene.codes.carriers);
  const cg=g.group(carriers,[[0,uniform],[6,scene.clamp],[7,ca.createView()],[9,scene.curl.createView()],[10,scene.repeat],[13,heads],[14,bands],[15,cb.createView()],[16,a.createView()],[17,b.createView()]]);
  params[22]=0;params[24]=0;params[26]=0;params[30]=0;params[39]=4;params[42]=256;
  let carrierError=0;const resetCases=[];
  for(const tick of [127,128,129,255,256,257]){
    params[40]=tick;d.queue.writeBuffer(uniform,0,params);e=d.createCommandEncoder();scene.compute(e,carriers,cg,[8,8,8]);const av=await download(a,e);e=d.createCommandEncoder();const bv=await download(b,e);
    const i=((4*8+4)*8+4)*4;
    const seed=[(4.5*187.5)/4800+.41,(4.5*181.25)/3200+.17,(4.5*187.5)/4200+.29];
    const expectedA=tick%256===0?seed:[2,2,2],expectedB=(tick+128)%256===0?seed:[3,3,3];
    for(let c=0;c<3;c++){carrierError=Math.max(carrierError,Math.abs(av[i+c]-expectedA[c]),Math.abs(bv[i+c]-expectedB[c]));}
    resetCases.push({tick,a:Array.from(av.slice(i,i+3)),b:Array.from(bv.slice(i,i+3)),expectedA,expectedB});
  }
  return {
    renewedCarrierReset:{maxError:carrierError,tolerance:.001,resetCases,scope:'Actual joint GPU producer, staggered reset and neighboring ticks; interior constant donor charts'},
    assembledCoreAffine:{maxError:affineError,tolerance:2e-5,actual,expected},
    mixCpuReference:{maxError:referenceError,tolerance:1,scope:'Each channel normalized by its declared half-float/age tolerance'},
    mixPhaseConservation:{maxError:massError,tolerance:.05},
    mixBirthConservation:{maxError:birthError,tolerance:birthRoundingBudget,relativeDrift:birthError/before[3],scope:'One local binary16 ULP per output plus 16 f32 epsilons of the maximum input; no exact conservation claim'},
    mixNoBirthInjection:{maxError:Math.max(...Array.from({length:N},(_,i)=>Math.abs(merged[i*4+3]))),tolerance:0},
    mixAgeMoment:{maxError:ageRelativeError,tolerance:.001},
    mixCreatesBridge:{maxError:components===1?0:1,tolerance:0,components,sourceSeconds:40,kappa:1000},
  };
}
