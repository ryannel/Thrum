struct Vertex { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32> }
@vertex fn vertex(@builtin(vertex_index) id:u32)->Vertex{let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));var v:Vertex;v.position=vec4<f32>(p[id],0.,1.);v.uv=vec2<f32>((p[id].x+1.)*.5,1.-(p[id].y+1.)*.5);return v;}
struct MediaOutput {@location(0) S:vec4<f32>,@location(1) T:vec4<f32>,@location(2) guide:vec4<f32>}
@fragment fn media(input:Vertex)->MediaOutput{let ray=rayDirection(input.uv);let h=sceneHit(scene.camera.xyz,ray,true);let r=transport(scene.camera.xyz,ray,h.t,scene.switches.x>.5);let cb=cloudInterval(scene.camera.xyz,ray,h.t);var out:MediaOutput;out.S=vec4<f32>(r.S*scene.display.x,fogTau(scene.camera.xyz,ray,h.t));out.T=vec4<f32>(r.T,dot(airTau(scene.camera.xyz,ray,h.t),vec3<f32>(.2126,.7152,.0722)));out.guide=vec4<f32>(h.t,f32(h.kind),max(0.,cb.x),max(0.,cb.y));if(!r.valid){out.S=vec4<f32>(1000.,0.,1000.,-1.);}return out;}
struct ReflectionOutput {@location(0) color:vec4<f32>,@location(1) guide:vec4<f32>,@location(2) cloudPath:vec4<f32>}
@fragment fn reflection(input:Vertex)->ReflectionOutput{let ray=rayDirection(input.uv);var out:ReflectionOutput;out.color=vec4<f32>(0.);out.guide=vec4<f32>(-1.);out.cloudPath=vec4<f32>(-1.);if(ray.y>=0.||scene.switches.y<.5){return out;}let t=-scene.camera.y/ray.y;let p=scene.camera.xyz+ray*t;if(p.x< -600.||p.x>1400.||p.z<180.||p.z>1000.){return out;}let rd=reflectedDirection(ray);let o=p+rd*.001;let h=sceneHit(o,rd,false);let v=transport(o,rd,h.t,true);var color=v.S+v.T*terminal(o,rd,h);if(!v.valid){color=vec3<f32>(10000000.,0.,10000000.);}out.color=vec4<f32>(color*scene.display.x,fogTau(o,rd,h.t));out.guide=vec4<f32>(h.t,f32(h.kind),t,dot(airTau(o,rd,h.t),vec3<f32>(.2126,.7152,.0722)));let cb=cloudInterval(o,rd,h.t);out.cloudPath=vec4<f32>(max(vec2<f32>(0.),cb),0.,0.);return out;}
@group(1) @binding(0) var mainS:texture_2d<f32>;
@group(1) @binding(1) var mainT:texture_2d<f32>;
@group(1) @binding(2) var mainGuide:texture_2d<f32>;
@group(1) @binding(3) var reflectedColor:texture_2d<f32>;
@group(1) @binding(4) var reflectedGuide:texture_2d<f32>;
@group(1) @binding(6) var reflectedCloudPath:texture_2d<f32>;
@group(1) @binding(5) var<storage,read_write> fallbackBits:array<atomic<u32>>;
fn reflectionValue(uv:vec2<f32>,p:vec3<f32>,ray:vec3<f32>,allowFallback:bool)->vec4<f32>{
 if(scene.switches.y<.5){return vec4<f32>(scene.sky.rgb*scene.sky.w*scene.display.x,1.);}
 let rd=reflectedDirection(ray);let o=p+rd*.001;let h=sceneHit(o,rd,false);let cb=cloudInterval(o,rd,h.t);let fog=fogTau(o,rd,h.t);let air=dot(airTau(o,rd,h.t),vec3<f32>(.2126,.7152,.0722));let dim=vec2<i32>(textureDimensions(reflectedColor));let pix=uv*vec2<f32>(dim)-.5;let lo=vec2<i32>(floor(pix));let f=fract(pix);var color=vec3<f32>(0.);var total=0.;
 for(var y=0;y<2;y++){for(var x=0;x<2;x++){let q=clamp(lo+vec2<i32>(x,y),vec2<i32>(0),dim-1);let g=textureLoad(reflectedGuide,q,0);let c=textureLoad(reflectedColor,q,0);let cp=textureLoad(reflectedCloudPath,q,0);if(abs(cp.x-max(0.,cb.x))<max(4.,.02*abs(cb.x))&&abs(cp.y-max(0.,cb.y))<max(4.,.02*abs(cb.y))&&g.x>=0.&&u32(g.y+.5)==h.kind&&abs(g.x-h.t)<=max(4.,.015*h.t)&&abs(c.a-fog)<.01&&abs(g.w-air)<.002){let w=select(1.-f.x,f.x,x==1)*select(1.-f.y,f.y,y==1);color+=c.rgb*w;total+=w;}}}
 if(scene.misc.w>.5){total=0.;}if(total>.9999){return vec4<f32>(color/total,1.);}if(!allowFallback){return vec4<f32>(0.);}
 let value=transport(o,rd,h.t,true);if(!value.valid){return vec4<f32>(1000.,0.,1000.,1.);}return vec4<f32>((value.S+value.T*terminal(o,rd,h))*scene.display.x,1.);
}
fn finalColor(uv:vec2<f32>,allowFallback:bool)->vec4<f32>{
 let ray=rayDirection(uv);let hit=sceneHit(scene.camera.xyz,ray,true);let fog=fogTau(scene.camera.xyz,ray,hit.t);let air=dot(airTau(scene.camera.xyz,ray,hit.t),vec3<f32>(.2126,.7152,.0722));let cb=cloudInterval(scene.camera.xyz,ray,hit.t);let dim=vec2<i32>(textureDimensions(mainS));let pix=uv*vec2<f32>(dim)-.5;let lo=vec2<i32>(floor(pix));let f=fract(pix);var S=vec3<f32>(0.);var T=vec3<f32>(0.);var total=0.;
 for(var y=0;y<2;y++){for(var x=0;x<2;x++){let q=clamp(lo+vec2<i32>(x,y),vec2<i32>(0),dim-1);let g=textureLoad(mainGuide,q,0);let s=textureLoad(mainS,q,0);let t=textureLoad(mainT,q,0);if(u32(g.y+.5)==hit.kind&&abs(g.x-hit.t)<=max(2.,.015*hit.t)&&abs(s.a-fog)<.01&&abs(t.a-air)<.002&&abs(g.z-max(0.,cb.x))<max(4.,.02*abs(cb.x))&&abs(g.w-max(0.,cb.y))<max(4.,.02*abs(cb.y))){let w=select(1.-f.x,f.x,x==1)*select(1.-f.y,f.y,y==1);S+=s.rgb*w;T+=t.rgb*w;total+=w;}}}
 if(scene.misc.w>.5){total=0.;}if(total>.9999){S/=total;T/=total;}else if(allowFallback){let v=transport(scene.camera.xyz,ray,hit.t,scene.switches.x>.5);if(!v.valid){return vec4<f32>(1000.,0.,1000.,1.);}S=v.S*scene.display.x;T=v.T;}else{return vec4<f32>(0.);}
 var background=vec3<f32>(0.);
 if(hit.kind==3u){let r=reflectionValue(uv,scene.camera.xyz+ray*hit.t,ray,allowFallback);if(r.a==0.){return vec4<f32>(0.);}background=r.rgb;}else{background=terminal(scene.camera.xyz,ray,hit)*scene.display.x;}
 return vec4<f32>(S+T*background,1.);
}
@fragment fn resolve(input:Vertex)->@location(0) vec4<f32>{let c=finalColor(input.uv,false);if(c.a==0.){let q=vec2<u32>(input.position.xy);let i=q.y*u32(scene.E.w)+q.x;atomicOr(&fallbackBits[i/32u],1u<<(i%32u));}return c;}
@fragment fn fallback(input:Vertex)->@location(0) vec4<f32>{let q=vec2<u32>(input.position.xy);let i=q.y*u32(scene.E.w)+q.x;if((atomicLoad(&fallbackBits[i/32u])&(1u<<(i%32u)))==0u){discard;}atomicAdd(&errors[1],1u);return finalColor(input.uv,true);}
@group(2) @binding(0) var hdr:texture_2d<f32>;
@fragment fn display(input:Vertex)->@location(0) vec4<f32>{var rgb=textureLoad(hdr,vec2<i32>(input.position.xy),0).rgb;rgb=max(rgb,vec3<f32>(0.));rgb=rgb/(1.+rgb);rgb=select(rgb*12.92,1.055*pow(rgb,vec3<f32>(1./2.4))-.055,rgb>vec3<f32>(.0031308));return vec4<f32>(rgb,1.);}
