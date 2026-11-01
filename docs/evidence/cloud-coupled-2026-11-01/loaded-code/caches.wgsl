@group(1) @binding(0) var depthOut:texture_storage_3d<rgba16float,write>;
@group(1) @binding(1) var ambientOut:texture_storage_3d<rgba16float,write>;
@group(1) @binding(2) var maxOut:texture_storage_3d<r32float,write>;
@compute @workgroup_size(8,8) fn prefix(@builtin(global_invocation_id) id:vec3<u32>){
 if(id.x>=130u||id.y>=130u){return;}
 if(id.x==0u||id.x==129u||id.y==0u||id.y==129u){for(var z=0;z<130;z++){textureStore(depthOut,vec3<i32>(i32(id.x),i32(id.y),z),vec4<f32>(0.));}return;}
 let uv=(vec2<f32>(id.xy)-.5)/128.;let center=scene.cloud.xyz+256.;let transverse=scene.basisU.xyz*((uv.x*2.-1.)*scene.basisU.w)+scene.basisV.xyz*((uv.y*2.-1.)*scene.basisV.w);
 let interval=rayBox(center+transverse-scene.basisW.xyz*scene.basisW.w,scene.basisW.xyz,scene.cloud.xyz+vec3<f32>(112.,32.,96.),scene.cloud.xyz+vec3<f32>(400.,256.,432.));let cell=2.*scene.basisW.w/128.;let sub=2u*u32(ceil(cell/8.));let ds=cell/f32(sub);var tau=vec3<f32>(0.);textureStore(depthOut,vec3<i32>(vec2<i32>(id.xy),0),vec4<f32>(0.));
 for(var z=0u;z<128u;z++){
 if(interval.y>interval.x&&f32(z+1u)*cell>=max(0.,interval.x)&&f32(z)*cell<=interval.y){for(var k=0u;k<sub;k++){let w=-scene.basisW.w+f32(z)*cell+(f32(k)+.5)*ds;let p=center+transverse+scene.basisW.xyz*w;tau+=vec3<f32>(sigmaCloud(p,0.),sigmaCloud(p,1.),sigmaCloud(p,2.))*ds;if(k+1u==sub/2u){textureStore(depthOut,vec3<i32>(vec2<i32>(id.xy),i32(z+1u)),vec4<f32>(tau,0.));}}}else{textureStore(depthOut,vec3<i32>(vec2<i32>(id.xy),i32(z+1u)),vec4<f32>(tau,0.));}
 }
 textureStore(depthOut,vec3<i32>(vec2<i32>(id.xy),129),vec4<f32>(tau,0.));
}
@compute @workgroup_size(4,4,4) fn ambient(@builtin(global_invocation_id) id:vec3<u32>){
 if(any(id>=vec3<u32>(32u))){return;}let p=scene.cloud.xyz+(vec3<f32>(id)+.5)*16.;var result=vec3<f32>(0.);if(any(p<scene.cloud.xyz+vec3<f32>(112.,32.,80.))||any(p>scene.cloud.xyz+vec3<f32>(400.,256.,432.))){textureStore(ambientOut,vec3<i32>(id),vec4<f32>(0.));return;}
 let dirs=array<vec3<f32>,5>(vec3<f32>(1.,0.,0.),vec3<f32>(-1.,0.,0.),vec3<f32>(0.,1.,0.),vec3<f32>(0.,0.,1.),vec3<f32>(0.,0.,-1.));
 for(var k=0;k<5;k++){let d=dirs[k];let hit=rayBox(p,d,scene.cloud.xyz+vec3<f32>(128.,48.,96.),scene.cloud.xyz+vec3<f32>(384.,240.,416.));let start=max(0.,hit.x);let length=max(0.,hit.y-start);let count=max(1u,u32(ceil(length/4.)));let h=length/f32(count);var tau=0.;for(var n=0u;n<count;n++){tau+=sigmaCloud(p+d*(start+(f32(n)+.5)*h),0.)*h;}result+=scene.sky.rgb*scene.sky.w*exp(-airTau(p,d,FAR)-vec3<f32>(fogTau(p,d,FAR)+tau))/6.;}
 textureStore(ambientOut,vec3<i32>(id),vec4<f32>(result/5000.,0.));
}
@compute @workgroup_size(4,4,4) fn maximum(@builtin(global_invocation_id) id:vec3<u32>){
 if(any(id>=vec3<u32>(32u))){return;}var v=0.;if(any(id<vec3<u32>(7u,2u,5u))||any(id>vec3<u32>(24u,15u,26u))){textureStore(maxOut,vec3<i32>(id),vec4<f32>(0.));return;}for(var z=-1;z<=4;z++){for(var y=-1;y<=4;y++){for(var x=-1;x<=4;x++){let q=vec3<i32>(id)*4+vec3<i32>(x,y,z);if(all(q>=vec3<i32>(0))&&all(q<vec3<i32>(128))){v=max(v,textureLoad(density,q,0).x);}}}}textureStore(maxOut,vec3<i32>(id),vec4<f32>(v,0.,0.,0.));
}
