// One scene state and one incident-light closure for surfaces and all media.
struct Scene {
 cloud:vec4<f32>, camera:vec4<f32>, lookAt:vec4<f32>, sun:vec4<f32>,
 basisU:vec4<f32>, basisV:vec4<f32>, basisW:vec4<f32>, sky:vec4<f32>,
 E:vec4<f32>, fogA:vec4<f32>, fogB:vec4<f32>, fogMaterial:vec4<f32>,
 switches:vec4<f32>, display:vec4<f32>, misc:vec4<f32>, spare:vec4<f32>
}
@group(0) @binding(0) var density:texture_3d<f32>;
@group(0) @binding(1) var depthCache:texture_3d<f32>;
@group(0) @binding(2) var ambientCache:texture_3d<f32>;
@group(0) @binding(3) var maxCache:texture_3d<f32>;
@group(0) @binding(4) var filtered:sampler;
@group(0) @binding(5) var<uniform> scene:Scene;
@group(0) @binding(6) var<storage,read_write> errors:array<atomic<u32>>;
const PI:f32=3.141592653589793;
const FAR:f32=4000.;
fn sigmaCloud(p:vec3<f32>,mip:f32)->f32 {
 let uv=(p-scene.cloud.xyz)/512.;if(any(uv<vec3<f32>(0.))||any(uv>vec3<f32>(1.))){return 0.;}
 return textureSampleLevel(density,filtered,uv,mip).x;
}
fn rayBox(o:vec3<f32>,d:vec3<f32>,lo:vec3<f32>,hi:vec3<f32>)->vec2<f32>{
 var a=-1e20;var b=1e20;
 for(var k=0;k<3;k++){if(abs(d[k])<1e-8){if(o[k]<lo[k]||o[k]>hi[k]){return vec2<f32>(1.,-1.);}}else{let x=(lo[k]-o[k])/d[k];let y=(hi[k]-o[k])/d[k];a=max(a,min(x,y));b=min(b,max(x,y));}}
 return vec2<f32>(a,b);
}
fn cloudInterval(o:vec3<f32>,d:vec3<f32>,terminalDistance:f32)->vec2<f32>{let b=rayBox(o,d,scene.cloud.xyz+vec3<f32>(128.,48.,96.),scene.cloud.xyz+vec3<f32>(384.,240.,416.));let near=max(0.,b.x);let far=min(terminalDistance,b.y);if(far<=near){return vec2<f32>(0.);}return vec2<f32>(near,far);}
fn sphereInterval(o:vec3<f32>,d:vec3<f32>,s:vec4<f32>)->vec2<f32>{let q=o-s.xyz;let b=dot(q,d);let c=dot(q,q)-s.w*s.w;let h=b*b-c;if(h<0.){return vec2<f32>(1.,-1.);}let r=sqrt(h);return vec2<f32>(-b-r,-b+r);}
fn fogTau(o:vec3<f32>,d:vec3<f32>,distance:f32)->f32 {
 let a=sphereInterval(o,d,scene.fogA);let b=sphereInterval(o,d,scene.fogB);
 return max(0.,min(distance,a.y)-max(0.,a.x))*scene.fogMaterial.x+max(0.,min(distance,b.y)-max(0.,b.x))*scene.fogMaterial.y;
}
fn airSigma(p:vec3<f32>)->vec3<f32>{return vec3<f32>(6e-6,12e-6,24e-6)*exp(-max(p.y,0.)/8000.)*scene.switches.w;}
fn airTau(o:vec3<f32>,d:vec3<f32>,distance:f32)->vec3<f32>{let k=d.y/8000.;var length=distance;if(abs(k)>1e-6){length=(1.-exp(-k*distance))/k;}return airSigma(o)*length;}
fn sunDepth(p:vec3<f32>)->vec3<f32>{
 let q=p-(scene.cloud.xyz+vec3<f32>(256.));
 var uv=vec3<f32>(dot(q,scene.basisU.xyz)/(2.*scene.basisU.w),dot(q,scene.basisV.xyz)/(2.*scene.basisV.w),dot(q,scene.basisW.xyz)/(2.*scene.basisW.w))+.5;
 if(uv.x<0.||uv.x>1.||uv.y<0.||uv.y>1.||uv.z<=0.){return vec3<f32>(0.);}
 uv.z=min(uv.z,1.);return textureSampleLevel(depthCache,filtered,(uv*128.+1.)/130.,0.).rgb;
}
fn geometryVisibility(p:vec3<f32>,towardLight:vec3<f32>)->f32{
 let a=sphereInterval(p+towardLight*.02,towardLight,vec4<f32>(70.,45.,15.,45.));if(a.y>0.&&a.x<FAR){return 0.;}
 if(scene.switches.z>.5){let b=sphereInterval(p+towardLight*.02,towardLight,vec4<f32>(-72.,385.,-89.,28.));if(b.y>0.&&b.x<FAR){return 0.;}}
 return 1.;
}
fn incident(p:vec3<f32>,tauCloud:f32)->vec3<f32>{return scene.E.rgb*scene.sun.w*geometryVisibility(p,scene.sun.xyz)*exp(-airTau(p,scene.sun.xyz,FAR)-vec3<f32>(fogTau(p,scene.sun.xyz,FAR)+tauCloud));}
fn hg(mu:f32,g:f32)->f32{let q=1.+g*g-2.*g*mu;return (1.-g*g)/(4.*PI*q*sqrt(q));}
fn cloudAmbient(p:vec3<f32>)->vec3<f32>{let uv=clamp((p-scene.cloud.xyz)/512.,vec3<f32>(0.),vec3<f32>(1.));return textureSampleLevel(ambientCache,filtered,uv,0.).rgb*5000.;}
struct Phases {cloud:vec4<f32>,air:f32,fog:f32}
fn phases(ray:vec3<f32>)->Phases{let mu=dot(ray,scene.sun.xyz);var p:Phases;p.cloud=vec4<f32>(hg(mu,.866617162492502),.8*hg(mu,.433308581246251),.64*hg(mu,.2166542906231255),.512*hg(mu,.10832714531156275));p.air=3.*(1.+mu*mu)/(16.*PI);p.fog=hg(mu,scene.fogMaterial.w);return p;}
struct Medium {ext:vec3<f32>,source:vec3<f32>}
fn medium(p:vec3<f32>,phase:Phases,cloudOn:bool)->Medium{
 var c=0.;if(cloudOn){c=sigmaCloud(p,0.);}
 var fog=0.;if(distance(p,scene.fogA.xyz)<scene.fogA.w){fog+=scene.fogMaterial.x;}if(distance(p,scene.fogB.xyz)<scene.fogB.w){fog+=scene.fogMaterial.y;}
 let a=airSigma(p);var result:Medium;result.ext=a+vec3<f32>(fog+c);result.source=vec3<f32>(0.);
 if(scene.sun.w>0.){let tau=sunDepth(p);let commonLight=incident(p,0.);let primary=exp(-tau.x);var cloud=phase.cloud.x*primary;
 if(c>0.){cloud+=phase.cloud.y*exp(-.2*tau.y)+phase.cloud.z*exp(-.04*tau.z)+phase.cloud.w*exp(-.008*tau.z);}
 result.source=commonLight*(a*phase.air*primary+vec3<f32>(fog*scene.fogMaterial.z*phase.fog*primary+c*cloud));}
 if(c>0.&&scene.sky.w>0.){result.source+=c*cloudAmbient(p);}return result;
}
struct Transport {S:vec3<f32>,T:vec3<f32>,valid:bool}
fn transport(o:vec3<f32>,ray:vec3<f32>,distance:f32,cloudOn:bool)->Transport{
 var cuts:array<f32,8>;cuts[0]=0.;cuts[1]=distance;
 let cb=rayBox(o,ray,scene.cloud.xyz+vec3<f32>(128.,48.,96.),scene.cloud.xyz+vec3<f32>(384.,240.,416.));
 let fa=sphereInterval(o,ray,scene.fogA);let fb=sphereInterval(o,ray,scene.fogB);
 cuts[2]=clamp(cb.x,0.,distance);cuts[3]=clamp(cb.y,0.,distance);cuts[4]=clamp(fa.x,0.,distance);cuts[5]=clamp(fa.y,0.,distance);cuts[6]=clamp(fb.x,0.,distance);cuts[7]=clamp(fb.y,0.,distance);
 for(var i=1;i<8;i++){let v=cuts[i];var j=i;loop{if(j==0||cuts[j-1]<=v){break;}cuts[j]=cuts[j-1];j--;}cuts[j]=v;}
 let phase=phases(ray);var result:Transport;result.S=vec3<f32>(0.);result.T=vec3<f32>(1.);result.valid=true;
 for(var segment=0;segment<7;segment++){
 var t=cuts[segment];let end=cuts[segment+1];if(end<=t){continue;}
 let mid=.5*(t+end);let inCloud=cloudOn&&mid>=cb.x&&mid<=cb.y;let inFog=(mid>=fa.x&&mid<=fa.y)||(mid>=fb.x&&mid<=fb.y);
 var count=0u;
 loop{
 if(t>=end){break;}if(count>=300u){atomicAdd(&errors[0],1u);result.valid=false;return result;}if(max(result.T.x,max(result.T.y,result.T.z))*800000.<1.){return result;}count++;
 var h=min(select(64.,8.,inFog),end-t);
 if(inCloud){
 let p=o+ray*(t+.001);let cell=clamp(vec3<i32>(floor((p-scene.cloud.xyz)/16.)),vec3<i32>(0),vec3<i32>(31));let maximum=textureLoad(maxCache,cell,0).x;
 if(maximum>0.){h=min(4.,end-t);}else{
 var next=1e20;for(var k=0;k<3;k++){if(abs(ray[k])>1e-8){let edge=scene.cloud[k]+(f32(cell[k])+select(0.,1.,ray[k]>0.))*16.;next=min(next,max(.001,(edge-p[k])/ray[k]));}}h=min(h,next+.001);
 }
 }
 let p=o+ray*(t+.5*h);let m=medium(p,phase,cloudOn);let tau=m.ext*h;let stepT=exp(-tau);let factor=h*select((vec3<f32>(1.)-stepT)/max(tau,vec3<f32>(1e-20)),vec3<f32>(1.)-.5*tau+tau*tau/6.-tau*tau*tau/24.,tau<vec3<f32>(.001));result.S+=result.T*m.source*factor;result.T*=stepT;t+=h;
 }
 }
 return result;
}
struct Hit {t:f32,kind:u32,normal:vec3<f32>}
fn sceneHit(o:vec3<f32>,ray:vec3<f32>,mirror:bool)->Hit{
 var h:Hit;h.t=FAR;h.kind=0u;h.normal=vec3<f32>(0.,1.,0.);
 if(ray.y< -1e-7){let t=-o.y/ray.y;if(t>0.&&t<h.t){h.t=t;h.kind=1u;let p=o+ray*t;if(mirror&&p.x>=-600.&&p.x<=1400.&&p.z>=180.&&p.z<=1000.){h.kind=3u;}}}
 let a=sphereInterval(o,ray,vec4<f32>(70.,45.,15.,45.));if(a.y>=a.x&&a.x>.01&&a.x<h.t){h.t=a.x;h.kind=2u;h.normal=normalize(o+ray*a.x-vec3<f32>(70.,45.,15.));}
 if(scene.switches.z>.5){let b=sphereInterval(o,ray,vec4<f32>(-72.,385.,-89.,28.));if(b.y>=b.x&&b.x>.01&&b.x<h.t){h.t=b.x;h.kind=4u;h.normal=normalize(o+ray*b.x-vec3<f32>(-72.,385.,-89.));}}
 return h;
}
fn surface(p:vec3<f32>,normal:vec3<f32>,kind:u32)->vec3<f32>{
 var albedo=vec3<f32>(.52,.56,.43);if(kind==1u){let q=fract(p.xz/100.);if(q.x<.025||q.y<.025){albedo*=.45;}}
 if(kind==2u){albedo=vec3<f32>(.65,.21,.09);}if(kind==4u){albedo=vec3<f32>(.24,.25,.27);}
 let light=incident(p+normal*.03,sunDepth(p).x);return albedo/PI*(light*max(0.,dot(normal,scene.sun.xyz))+scene.sky.rgb*scene.sky.w*.5);
}
fn rayDirection(uv:vec2<f32>)->vec3<f32>{let f=normalize(scene.lookAt.xyz-scene.camera.xyz);let r=normalize(cross(f,vec3<f32>(0.,1.,0.)));let u=cross(r,f);let q=vec2<f32>((uv.x*2.-1.)*scene.camera.w,1.-uv.y*2.);return normalize(f+scene.lookAt.w*(q.x*r+q.y*u));}
fn reflectedDirection(ray:vec3<f32>)->vec3<f32>{return vec3<f32>(ray.x,-ray.y,ray.z);}
fn terminal(o:vec3<f32>,ray:vec3<f32>,hit:Hit)->vec3<f32>{if(hit.kind==0u){return scene.sky.rgb*scene.sky.w;}return surface(o+ray*hit.t,hit.normal,hit.kind);}
