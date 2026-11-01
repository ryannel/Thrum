// Canonical prepared-source WGSL, shared by Q1 and coupled proofs.
export const SOURCE_WGSL = `
struct Params { headA:u32, bodyA:u32, headB:u32, bodyB:u32, flowOffset:u32, side:u32, theta:f32, duration:f32, scales:vec4<f32>, mode:u32, pad0:u32, pad1:u32,pad2:u32 }
@group(0) @binding(0) var<storage,read> density:array<u32>;
@group(0) @binding(1) var<storage,read> flow:array<u32>;
@group(0) @binding(2) var<uniform> p:Params;
@group(0) @binding(3) var<storage,read_write> result:array<u32>;
fn byteAt(i:u32)->u32 { return (density[i/4u] >> ((i%4u)*8u))&255u; }
fn wordAt(i:u32)->u32 {return density[i/4u];}
fn voxel(c:vec3<i32>,head:u32,body:u32)->f32 {
 if(any(c<vec3<i32>(0))||any(c>=vec3<i32>(i32(p.side)))){return 0.;}
 let q=vec3<u32>(c);let n=p.side/8u;
 let brick=(q.z/8u*n+q.y/8u)*n+q.x/8u;
 let pos=head+brick*2u;let index=byteAt(pos)|(byteAt(pos+1u)<<8u);
 if(index==65535u){return 0.;}
 let local=(q.z%8u*8u+q.y%8u)*8u+q.x%8u;
 var code=0u;
 if(p.pad1==1u){
 let pointer=wordAt(body+index*4u);let block=head+(pointer&0xfffffffeu);
 if((pointer&1u)==0u){code=byteAt(block+local);}else{
 let wi=local/32u;let bit=local%32u;let mask=wordAt(block+wi*4u);
 if((mask&(1u<<bit))==0u){return 0.;}
 let prefixAt=block+64u+(local/64u)*2u;
 var rank=byteAt(prefixAt)|(byteAt(prefixAt+1u)<<8u);
 if((wi&1u)==1u){rank+=countOneBits(wordAt(block+(wi-1u)*4u));}
 rank+=countOneBits(mask&((1u<<bit)-1u));code=byteAt(block+80u+rank);
 }}else{code=byteAt(body+index*512u+local);}
 return f32(code)*(p.scales.w/255.);
}
fn sigma(q:vec3<f32>,head:u32,body:u32)->f32 {
 let xyz=q*f32(p.side)/512.-.5;let lo=vec3<i32>(floor(xyz));let f=fract(xyz);var total=0.;
 for(var z=0;z<2;z++){for(var y=0;y<2;y++){for(var x=0;x<2;x++){
 let r=vec3<i32>(x,y,z);let w=select(vec3<f32>(1.)-f,f,vec3<bool>(x==1,y==1,z==1));
 total+=voxel(lo+r,head,body)*w.x*w.y*w.z;
 }}}return total;
}
fn velocity(q:vec3<f32>)->vec3<f32>{
 let xyz=clamp(q*(32./512.)-.5,vec3<f32>(0.),vec3<f32>(31.));let lo=vec3<u32>(floor(xyz));let hi=min(lo+1u,vec3<u32>(31u));let f=fract(xyz);var v=vec3<f32>(0.);
 for(var z=0u;z<2u;z++){for(var y=0u;y<2u;y++){for(var x=0u;x<2u;x++){
 let c=select(lo,hi,vec3<bool>(x==1u,y==1u,z==1u));let bits=flow[p.flowOffset+(c.z*32u+c.y)*32u+c.x];
 let signed=vec3<i32>(i32(bits<<24u)>>24,i32(bits<<16u)>>24,i32(bits<<8u)>>24);
 let w=select(vec3<f32>(1.)-f,f,vec3<bool>(x==1u,y==1u,z==1u));
 v+=vec3<f32>(signed)/127.*p.scales.xyz*w.x*w.y*w.z;
 }}}return v;
}
fn evaluate(index:u32)->f32 {
 let n=p.side;let q=(vec3<f32>(f32(index%n),f32((index/n)%n),f32(index/(n*n)))+.5)*(512./f32(n));
 if(p.pad0==1u){let c=vec3<i32>(i32(index%n),i32((index/n)%n),i32(index/(n*n)));return mix(voxel(c,p.headA,p.bodyA),voxel(c,p.headB,p.bodyB),p.theta);}
 let v=velocity(q);let a=-p.theta*p.duration;let b=(1.-p.theta)*p.duration;
 let qa=q+a*velocity(q+.5*a*v);let qb=q+b*velocity(q+.5*b*v);
 return (1.-p.theta)*sigma(qa,p.headA,p.bodyA)+p.theta*sigma(qb,p.headB,p.bodyB);
}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let index=id.x*2u;if(index>=p.side*p.side*p.side){return;}
 result[id.x]=pack2x16float(vec2<f32>(evaluate(index),evaluate(index+1u)));
}`;
