// Reuse the pinned Q1 master as a held geometry comparator, not a new family pass.
import {SOURCE_WGSL} from '../cloud_source_proof/source.js';
import {ScalarField,FIELD_PARAMETERS_WGSL,FIELD_DISPATCH_WGSL} from './scalar-field.js';
import {compileShader,hash} from './gpu.js';
const MANIFEST='2fca53ff92a9787a0bb54bd9db0c470f5cfe2a5d755534c134eec38568acb739';
const DENSITY='999cd89480371d02dca2f7553b938e5b93e23f0b31ca9b306ae60beb62e325bd';
const pop=x=>{x-=x>>>1&0x55555555;x=(x&0x33333333)+(x>>>2&0x33333333);return Math.imul((x+(x>>>4)&0x0f0f0f0f),0x01010101)>>>24;};
export class PreparedCu {
  static async create(device,side=128) {
    const result=new PreparedCu();result.device=device;result.kind='prepared';result.maxLevel=0;
    try {
      const url='/docs/evidence/cloud-source-2026-11-01/selected-clip/';
      const manifest=new Uint8Array(await(await fetch(url+'cloudclip.json')).arrayBuffer());
      if(await hash(manifest)!==MANIFEST)throw Error('Prepared manifest identity mismatch');
      result.metadata=JSON.parse(new TextDecoder().decode(manifest));
      result.bytes=new Uint8Array(await(await fetch(url+'density.bin')).arrayBuffer());
      if(await hash(result.bytes)!==DENSITY)throw Error('Prepared density identity mismatch');
      result.data=new DataView(result.bytes.buffer);result.sigmaMax=result.metadata.runtime_sigma_max_per_m;
      result.recipe={id:'retained-q1-comparator',variant:0,seed:0,name:'retained Q1 detailed master',duration_s:240,reference_time_s:90,bounds:{min:[0,0,0],extent:[512,512,512]},inspection_centre_m:[256,144,256],manifest_sha256:MANIFEST,density_sha256:DENSITY,scope:'Static detailed master with retained authored finite lifecycle; no mature replacement or new family acceptance'};
      result.field=await ScalarField.create(device,[side,side,side]);
      result.raw=device.createBuffer({size:result.bytes.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
      result.uniform=device.createBuffer({size:64,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
      device.queue.writeBuffer(result.raw,0,result.bytes);
      result.shader=SOURCE_WGSL.slice(0,SOURCE_WGSL.indexOf('@compute')).replace('@binding(0)','@binding(6)').replace('@binding(1)','@binding(7)').replace('@binding(2)','@binding(8)').replace('@group(0) @binding(3) var<storage,read_write> result:array<u32>;','')+`
fn canonicalSigma(q:vec3f,age:f32)->f32 {
 if(age<=0.||age>=240.){return 0.;}
 return mix(sigma(q,p.headA,p.bodyA),sigma(q,p.headB,p.bodyB),p.theta);
}`;
      result.pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,FIELD_PARAMETERS_WGSL+result.shader+FIELD_DISPATCH_WGSL),entryPoint:'fillField'}});
      result.group=device.createBindGroup({layout:result.pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:result.field.levels[0].uniform}},{binding:1,resource:result.field.levels[0].texture.createView()},...result.sourceEntries()]});
      return result;
    } catch(error){result.destroy();throw error;}
  }
  bracket(t){const a=this.metadata.source_times_s;let lo=0,hi=a.length-1;while(lo+1<hi){const m=(lo+hi)>>1;if(a[m]<=t)lo=m;else hi=m;}return [lo,lo+1,(t-a[lo])/(a[lo+1]-a[lo])];}
  prepare(time){
    const t=Math.fround(time);if(this.frame?.sourceParams[0]===t)return this.frame;
    if(!Number.isFinite(t)||t<0||t>240)throw Error('Prepared clip outside finite interval');
    const [ia,ib,theta]=this.bracket(t),a=this.metadata.density_frames[ia],b=this.metadata.density_frames[ib];
    const data=new ArrayBuffer(64),u=new Uint32Array(data),f=new Float32Array(data);
    u[0]=a.offset;u[1]=a.offset+a.pointer_table_offset;u[2]=b.offset;u[3]=b.offset+b.pointer_table_offset;u[5]=128;f[6]=theta;f[7]=this.metadata.source_times_s[ib]-this.metadata.source_times_s[ia];f[11]=this.sigmaMax;u[13]=1;u[14]=1;
    this.device.queue.writeBuffer(this.uniform,0,data);this.field.setParameters([0,0,0],[512,512,512],t);
    this.frame={sourceParams:new Float32Array([t,0,0,0]),stats:{profile:'retained Q1 static geometry comparator',encoded_source_bytes:this.bytes.byteLength}};this.needsBuild=true;return this.frame;
  }
  sourceEntries(){return [{binding:6,resource:{buffer:this.raw}},{binding:8,resource:{buffer:this.uniform}}];}
  // Direct mode never calls velocity(), so the unused flow binding is omitted.
  probeEntries(){return [{binding:0,resource:{buffer:this.field.levels[0].uniform}},...this.sourceEntries()];}
  voxel(c,frame){
    if(c.some(x=>x<0||x>=128))return 0;
    const [x,y,z]=c,brick=((z>>3)*16+(y>>3))*16+(x>>3),head=frame.offset;
    const index=this.data.getUint16(head+brick*2,true);if(index===65535)return 0;
    const local=((z&7)*8+(y&7))*8+(x&7),pointer=this.data.getUint32(head+frame.pointer_table_offset+index*4,true),block=head+(pointer&0xfffffffe);
    let code;
    if(!(pointer&1))code=this.bytes[block+local];
    else {const wi=local>>>5,bit=local&31,mask=this.data.getUint32(block+wi*4,true);if(!(mask&(1<<bit)))return 0;let rank=this.data.getUint16(block+64+(local>>>6)*2,true);if(wi&1)rank+=pop(this.data.getUint32(block+(wi-1)*4,true));rank+=pop(mask&((2**bit-1)>>>0));code=this.bytes[block+80+rank];}
    return code*(this.sigmaMax/255);
  }
  sample(q,time){if(time<=0||time>=240)return 0;const [ia,ib,theta]=this.bracket(time),frames=this.metadata.density_frames;const xyz=q.map(x=>x/4-.5),lo=xyz.map(Math.floor),f=xyz.map((x,k)=>x-lo[k]);let value=0;for(let z=0;z<2;z++)for(let y=0;y<2;y++)for(let x=0;x<2;x++){const c=[lo[0]+x,lo[1]+y,lo[2]+z],w=(x?f[0]:1-f[0])*(y?f[1]:1-f[1])*(z?f[2]:1-f[2]);value+=w*((1-theta)*this.voxel(c,frames[ia])+theta*this.voxel(c,frames[ib]));}return value;}
  encode(pass){if(this.needsBuild)this.field.encode(pass,this.pipeline,this.group);}
  markSubmitted(){this.needsBuild=false;}
  destroy(){this.field?.destroy();this.raw?.destroy();this.uniform?.destroy();}
}
