struct Transport  {
  s:vec3f,t:vec3f
}
;
fn beer(source:vec3f,sigma:vec3f,ds:f32)->Transport {
  let t=exp(-sigma*ds);
  return Transport(source*(1.-t)/max(sigma,vec3f(1e-10)),t);
}
fn join(a:Transport,b:Transport)->Transport {
  return Transport(a.s+a.t*b.s,a.t*b.t);
}
fn fogSigma(x:vec3f)->f32 {
  if(p.image.w<.5) {
    return 0.;
  }
  let ell=(x-vec3f(-1400.,120.,900.))/vec3f(1900.,500.,1800.);
  return .0007*max(0.,1.-dot(ell,ell));
}
fn fogSun(x:vec3f)->f32 {
  if(p.image.w<.5) {
    return 1.;
  }
  let iv=rayBox(x,p.sun.xyz,vec3f(-3300.,0.,-900.),vec3f(500.,620.,2700.));
  let a=max(iv.x,0.);
  let b=iv.y;
  if(b<=a) {
    return 1.;
  }
  let ds=(b-a)/8.;
  var tau=0.;
  for(var i=0u;i<8u;i++) {
    tau+=fogSigma(x+p.sun.xyz*(a+(f32(i)+.5)*ds))*ds;
  }
  return exp(-tau);
}
fn rayAir(o:vec3f,d:vec3f,a:f32,b:f32,count:u32)->Transport {
  var r=Transport(vec3f(0),vec3f(1));
  let mu=dot(d,p.sun.xyz);
  for(var i=0u;i<count;i++) {
    let f0=f32(i)/f32(count);
    let f1=f32(i+1u)/f32(count);
    let t0=a+(b-a)*f0*f0;
    let t1=a+(b-a)*f1*f1;
    let point=o+d*((t0+t1)*.5);
    let h=length(point+vec3f(0,6371000.,0))-6371000.;
    let air=airAt(vec3f(0,h,0));
    let jr=3.*(1.+mu*mu)/(16.*PI);
    let fog=fogSigma(point);
    var visibility=1.;
    if(fog>0.) {
      visibility=cacheT(point).x;
    }
    let sunT=fogSun(point);
    let j=(air[1]*jr+air[2]*hg(.8,mu))*solar()*airSun(point)*sunT;
    let jf=fog*.9*(solar()*airSun(point)*visibility*sunT*hg(.3,mu)+skyMean()*.4);
    r=join(r,beer(j+jf,air[0]+vec3f(fog),t1-t0));
  }
  return r;
}
fn receiverT(x:vec3f)->f32 {
  if(p.domain.w>1.5) {
    return 1.;
  }
  if(p.domain.w>.5) {
    let drift=p.weather.w;
    let v=.55+.35*sin((x.x-drift)*.0012)*cos(x.z*.001);
    return exp(-max(0.,v)*1.8);
  }
  let center=(p.bmin.xyz+p.bmax.xyz)*.5;
  let extent=(p.bmax.xyz-p.bmin.xyz)*2.;
  let uv=(x.xz-center.xz)/extent.xz+.5;
  if(any(uv<vec2f(0))||any(uv>vec2f(1))) {
    return directT(x).x;
  }
  return textureSampleLevel(receiverMap,linearClamp,uv,0.).x;
}
fn ground(x:vec3f,d:vec3f)->vec3f {
  let sunT=receiverT(x)*fogSun(x);
  let checker=select(.12,.16,((i32(floor(x.x/1200.))+i32(floor(x.z/1200.)))&1)==0);
  let base=vec3f(checker*.65,checker*.9,checker);
  return base*(solar()*airSun(x)*max(p.sun.y,0.)*sunT/PI+skyMean()*.45);
}
fn integrate(o:vec3f,d:vec3f,terminal:f32,cloudEnabled:bool)->Transport {
  let box=rayBox(o,d,p.bmin.xyz,p.bmax.xyz);
  let start=max(0.,box.x);
  let end=min(box.y,terminal);
  var transport=Transport(vec3f(0),vec3f(1));
  let mode=u32(p.focus.w);
  if(end>start&&p.domain.w<.5&&cloudEnabled) {
    transport=rayAir(o,d,0.,start,8u);
    let ds=(end-start)/96.;
    let mu=dot(d,p.sun.xyz);
    for(var i=0u;i<96u;i++) {
      let x=o+d*(start+(f32(i)+.5)*ds);
      let coeff=coefficients(x,1.);
      let sigma=coeff.x+coeff.y;
      let air=airAt(x);
      let fog=fogSigma(x);
      var source=vec3f(0);
      if(sigma>.000005) {
        let ice=coeff.y/max(sigma,.000001);
        let vis=cloudSun(x);
        let sunlight=solar()*airSun(x)*fogSun(x);
        let direct=sunlight*(phase(mu,ice,1.)*vis.x+p.sun.w*phase(mu,ice,p.domain.z)*vis.y);
        let ambient=skyMean()*.5*(.2+.8*ambientA(x));
        source=sigma*.995*(direct+ambient);
        if(mode==2u) {
          source=sigma*.995*direct;
        }
        if(mode==3u) {
          source=vec3f(sigma*ambientA(x));
        }
        if(mode==4u) {
          let age=rawQ(x).z/160.;
          source=sigma*mix(vec3f(.2,.8,.95),vec3f(1.,.4,.15),clamp(age,0.,1.));
        }
      }
      let cloudVis=cacheT(x).x;
      let jAir=(air[1]*(3.*(1.+mu*mu)/(16.*PI))+air[2]*hg(.8,mu))*solar()*airSun(x)*cloudVis*fogSun(x);
      let jFog=fog*.9*(solar()*airSun(x)*cloudVis*hg(.3,mu)+skyMean()*.4);
      transport=join(transport,beer(source+jAir+jFog,vec3f(sigma+fog)+air[0],ds));
      if(max(max(transport.t.x,transport.t.y),transport.t.z)<.004) {
        break;
      }
    }
    if(max(max(transport.t.x,transport.t.y),transport.t.z)>.004) {
      transport=join(transport,rayAir(o,d,max(end,0.),terminal,24u));
    }
  }
  else {
    transport=rayAir(o,d,0.,terminal,36u);
  }
  return transport;
}
fn skyBoundary(d:vec3f)->vec3f {
  let disk=smoothstep(cos(.0048),cos(.0042),dot(d,p.sun.xyz));
  return solar()*(disk/.0000685);
}
fn skyRay(o:vec3f,d:vec3f)->vec3f {
  let ray=integrate(o,d,atmosphereExit(o,d),true);
  return ray.s+ray.t*skyBoundary(d);
}
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
  let a=array<vec2f,3>(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));
  return vec4f(a[i],0,1);
}
@fragment fn fs(@builtin(position) frag:vec4f)->@location(0) vec4f {
  let uv=frag.xy/p.image.xy;
  let aspect=p.image.x/p.image.y;
  let f=normalize(p.focus.xyz-p.camera.xyz);
  let r=normalize(cross(f,vec3f(0,1,0)));
  let u=cross(r,f);
  let xy=(uv*2.-1.)*vec2f(aspect,-1.)*p.image.z;
  let d=normalize(f+r*xy.x+u*xy.y);
  let mode=u32(p.focus.w);
  var color=vec3f(0);
  if(mode==1u) {
    let x=mix(p.bmin.x,p.bmax.x,uv.x);
    let y=mix(p.bmax.y,p.bmin.y,uv.y);
    let q=rawQ(vec3f(x,y,0));
    color=vec3f(q.x,q.y,q.w*25.);
    return vec4f(color,1);
  }
  let top=atmosphereExit(p.camera.xyz,d);
  var terminal=top;
  var hit=false;
  if(d.y<-.0001) {
    let tg=-p.camera.y/d.y;
    if(tg>0.&&tg<terminal) {
      terminal=tg;
      hit=true;
    }
  }
  let near=integrate(p.camera.xyz,d,terminal,p.front.w>.5);
  var background=skyBoundary(d);
  if(hit) {
    let pt=p.camera.xyz+d*terminal;
    background=ground(pt,d);
    if(p.camera.w>.5) {
      let reflected=reflect(d,vec3f(0,1,0));
      let mirror=skyRay(pt+vec3f(0,.05,0),reflected);
      let fresnel=.045+.955*pow(1.-max(dot(-d,vec3f(0,1,0)),0.),5.);
      background=mix(background,mirror,fresnel*.92);
    }
  }
  color=near.s+near.t*background;
  return vec4f(min(color*.5,vec3f(60000.)),1);
}
