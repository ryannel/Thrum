import {SOURCE_WGSL} from '../cloud_source_proof/source.js';
export {SOURCE_WGSL};
export async function hash(bytes) {
  return [...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(x=>x.toString(16).padStart(2,'0')).join('');
}
export async function module(device,code) {
  const m=device.createShaderModule({code});const info=await m.getCompilationInfo();
  const errors=info.messages.filter(x=>x.type==='error');
  if(errors.length)throw Error(errors.map(x=>`${x.lineNum}:${x.linePos} ${x.message}`).join('\n'));
  return m;
}
// Decoder mathematics are unchanged; only the output adapter differs from Q1.
export const FIELD_WGSL=SOURCE_WGSL.slice(0,SOURCE_WGSL.indexOf('@compute')).replace('var<storage,read_write> result:array<u32>;', 'var result:texture_storage_3d<rgba16float,write>;')+`
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){let i=id.x;let n=p.side;if(i>=n*n*n){return;}let c=vec3<i32>(i32(i%n),i32((i/n)%n),i32(i/(n*n)));var v=0.;if(all(c>=vec3<i32>(32,12,24))&&all(c<vec3<i32>(96,60,104))){v=evaluate(i);}textureStore(result,c,vec4<f32>(unpack2x16float(pack2x16float(vec2<f32>(v,0.))).x,0.,0.,0.));}`;
const MIP_WGSL=`
struct P {side:u32,p0:u32,p1:u32,p2:u32}
@group(0) @binding(0) var previous:texture_3d<f32>;
@group(0) @binding(1) var outputField:texture_storage_3d<rgba16float,write>;
@group(0) @binding(2) var<uniform> p:P;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){let i=id.x;let n=p.side;if(i>=n*n*n){return;}let c=vec3<i32>(i32(i%n),i32((i/n)%n),i32(i/(n*n)));var v=0.;for(var z=0;z<2;z++){for(var y=0;y<2;y++){for(var x=0;x<2;x++){v+=textureLoad(previous,c*2+vec3<i32>(x,y,z),0).x;}}}textureStore(outputField,c,vec4<f32>(unpack2x16float(pack2x16float(vec2<f32>(v*.125,0.))).x,0.,0.,0.));}`;
export class PreparedSource {
  static async create(device,url) {
    const result=new PreparedSource();result.device=device;
    const response=await fetch(new URL('cloudclip.json',url));const manifest=await response.text();result.manifestSha=await hash(new TextEncoder().encode(manifest));
    if(result.manifestSha!=='2fca53ff92a9787a0bb54bd9db0c470f5cfe2a5d755534c134eec38568acb739')throw Error('Uncertified source manifest for the pinned support');result.metadata=JSON.parse(manifest);
    const m=result.metadata;
    if(m.reader_kind!=='direct'||m.density_side!==128||m.density_encoding_version!=='thrum.cloud-source-proof.mixed-bricks.v1')throw Error('Unsupported source');
    result.bytes=new Uint8Array(await(await fetch(new URL('density.bin',url))).arrayBuffer());
    result.sha=await hash(result.bytes);
    if(result.sha!=='999cd89480371d02dca2f7553b938e5b93e23f0b31ca9b306ae60beb62e325bd')throw Error('Uncertified source: this bounded instrument only admits the pinned Q1 support');
    if(result.sha!==m.products['density.bin'].sha256)throw Error('Source hash mismatch');
    result.buffer=device.createBuffer({size:result.bytes.length,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    for(let at=0;at<result.bytes.length;at+=2*1024*1024){device.queue.writeBuffer(result.buffer,at,result.bytes.subarray(at,Math.min(at+2*1024*1024,result.bytes.length)));await device.queue.onSubmittedWorkDone();}
    result.dummy=device.createBuffer({size:4,usage:GPUBufferUsage.STORAGE});
    result.uniform=device.createBuffer({size:64,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});

    result.texture=device.createTexture({size:[128,128,128],mipLevelCount:3,dimension:'3d',format:'rgba16float',usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.COPY_SRC|GPUTextureUsage.STORAGE_BINDING});
    result.pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await module(device,FIELD_WGSL),entryPoint:'main'}});
    result.group=device.createBindGroup({layout:result.pipeline.getBindGroupLayout(0),entries:[result.buffer,result.dummy,result.uniform].map((buffer,binding)=>({binding,resource:{buffer}})).concat([{binding:3,resource:result.texture.createView({baseMipLevel:0,mipLevelCount:1})}])});
    result.mipPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await module(device,MIP_WGSL),entryPoint:'main'}});
    result.mips=[];
    for(let level=1;level<3;level++){
      const side=128>>level,uniform=device.createBuffer({size:16,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});device.queue.writeBuffer(uniform,0,new Uint32Array([side,0,0,0]));
      const group=device.createBindGroup({layout:result.mipPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:result.texture.createView({baseMipLevel:level-1,mipLevelCount:1})},{binding:1,resource:result.texture.createView({baseMipLevel:level,mipLevelCount:1})},{binding:2,resource:{buffer:uniform}}]});result.mips.push({side,level,group});
    }
    return result;
  }
  parameters(age) {
    const m=this.metadata,t=m.source_times_s;let lo=0,hi=t.length-1;
    while(lo+1<hi){const mid=(lo+hi)>>1;if(t[mid]<=age)lo=mid;else hi=mid;}
    const a=m.density_frames[lo],b=m.density_frames[lo+1],data=new ArrayBuffer(64),u=new Uint32Array(data),f=new Float32Array(data);
    u[0]=a.offset;u[1]=a.offset+a.pointer_table_offset;u[2]=b.offset;u[3]=b.offset+b.pointer_table_offset;u[5]=128;f[6]=(age-t[lo])/(t[lo+1]-t[lo]);f[7]=t[lo+1]-t[lo];f[11]=m.runtime_sigma_max_per_m;u[13]=1;u[14]=1;return data;
  }
  encodeInto(pass,age) {
    this.device.queue.writeBuffer(this.uniform,0,this.parameters(age));pass.setPipeline(this.pipeline);pass.setBindGroup(0,this.group);pass.dispatchWorkgroups(32768);
    for(const mip of this.mips){pass.setPipeline(this.mipPipeline);pass.setBindGroup(0,mip.group);pass.dispatchWorkgroups(Math.ceil(mip.side**3/64));}
  }
}
