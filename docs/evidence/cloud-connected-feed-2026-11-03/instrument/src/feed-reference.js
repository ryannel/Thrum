// Requested GPU producer checks with an independent finite-difference curve derivative.
export async function runFeedReference(scene){
 const g=scene.gpu,d=g.device;
 const uniform=g.buffer('feed oracle uniform',176,GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST);
 const heads=g.buffer('feed oracle heads',48*64,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST);
 const bands=g.buffer('feed oracle bands',32*49*4,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST);
 const result=g.buffer('feed oracle result',16,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC);
 const read=g.buffer('feed oracle read',16,GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST);
 const params=new Float32Array(44),hp=new Float32Array(48*16),bp=new Uint32Array(32*49);
 params.set([64,.25,3,192],20);params.set([.85,.08,1,1],24);params.set([1e6,1,3,1],28);
 const code=scene.codes.source.replace('fn main(','fn sourceStep(')+`@group(0) @binding(15) var<storage,read_write> r:array<vec4f>;@compute @workgroup_size(1) fn main(){r[0]=flow(p.camera.xyz);}`;
 const pipeline=await g.compute('assembled connected-feed oracle',code),bg=g.group(pipeline,[[0,uniform],[9,scene.curl.createView()],[10,scene.repeat],[13,heads],[14,bands],[15,result]]);
 function pack(n,U=28,A=24){hp.fill(0);bp.fill(0);for(let i=0;i<n;i++)hp.set([600,5000,-100,1500,A,900,1,U,i+1,1,0,1,U,.08*U/1212,.14*U/900,1],i*16);for(let j=0;j<32;j++){bp[j*49]=n;for(let i=0;i<n;i++)bp[j*49+1+i]=i;}d.queue.writeBuffer(heads,0,hp);d.queue.writeBuffer(bands,0,bp);}
 async function query(point,profile,lift){params.set(point,0);params[30]=profile;params[26]=lift;d.queue.writeBuffer(uniform,0,params);const e=d.createCommandEncoder();scene.compute(e,pipeline,bg,[1,1,1]);e.copyBufferToBuffer(result,0,read,0,16);g.submit(e);await read.mapAsync(GPUMapMode.READ);const a=Array.from(new Float32Array(read.getMappedRange().slice(0)));read.unmap();return a;}
 const root=[192,70.4],rootDot=[3,1.1],H=[600,5000,-100],U=28;
 const shear=(5000-1100)/8000,headDot=[3*(1+.6*shear),1.1+3*shear];
 function center(y,t){const sy=Math.max(0,Math.min(1,(y-1320)/(H[1]+U*t-1320))),f=sy*sy*(3-2*sy);return [0,1].map(j=>{const R=root[j]+rootDot[j]*t;return R+f*(H[j===0?0:2]+headDot[j]*t-R);});}
 const y=2500,C=center(y,0),eps=.001;
 const before=center(y,-eps),after=center(y,eps),lower=center(y-eps,0),upper=center(y+eps,0);
 const xz=[0,1].map(j=>(after[j]-before[j])/(2*eps)+U*(upper[j]-lower[j])/(2*eps));
 const expected=[xz[0],28,xz[1],28];pack(1);const actual=await query([C[0],y,C[1]],3,1);
 const curveError=Math.max(...actual.map((v,i)=>Math.abs(v-expected[i])));
 let releaseError=0;const releaseCases=[];
 for(const n of [1,4,6]){
  pack(n,14,0);const free=await query(H,1,.25),protectedFlow=await query(H,2,.25),released=await query(H,3,.25);
  const expectedRelease=free.map((v,i)=>(v+protectedFlow[i])*.5);
  releaseError=Math.max(releaseError,...released.map((v,i)=>Math.abs(v-expectedRelease[i])));
  pack(n,0,0);const zero=await query(H,3,0);pack(0,0,0);const unprotected=await query(H,3,0);
  releaseError=Math.max(releaseError,...zero.map((v,i)=>Math.abs(v-unprotected[i])));
  releaseCases.push({heads:n,free,protectedFlow,released,expected:expectedRelease,zero,unprotected});
 }
 return {connectedStemDerivative:{maxError:curveError,tolerance:2e-5,actual,expected,scope:'Actual full GPU flow versus independent finite differences of the time-dependent cubic center, isolated full stem'},overlapShutdownRelease:{maxError:releaseError,tolerance:2e-5,releaseCases,scope:'Actual one/four/six-head flow; half activity and zero activity after normalized composition, roll amplitude0 isolates protection'}};
}
