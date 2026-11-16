// Queries invert the map; the drawn cache below evaluates the rest sheet once.
fn forward_long(point:vec2<f32>,max_k:f32)->Field {
    var displacement=vec3(0.0); var gradient=vec2(0.0);
    var differential=vec3(0.0); var velocity=vec3(0.0);
    for(var j=0u;j<p.time.y;j++) {
        let c=components[j];let k=c.physical.xy;let magnitude=length(k);
        if magnitude<=max_k {
            let angle=long_angle(k,point,vec2(0.0),c.physical.w);
            let s=sin(angle);let co=cos(angle);let amp=c.physical.z;
            displacement+=amp*vec3(-p.physical.x*k.x/magnitude*s,co,-p.physical.x*k.y/magnitude*s);
            gradient-=amp*k*s;
            differential-=p.physical.x*amp/magnitude*vec3(k.x*k.x,k.x*k.y,k.y*k.y)*co;
            velocity+=amp*c.rate.x*vec3(p.physical.x*k.x/magnitude*co,s,p.physical.x*k.y/magnitude*co);
        }
    }
    let jac=(1.0+differential.x)*(1.0+differential.z)-differential.y*differential.y;
    return Field(vec4(displacement,jac),vec4(gradient,differential.xy),vec4(velocity,differential.z));
}
fn long_angle(k:vec2<f32>,point:vec2<f32>,offset:vec2<f32>,phase:f32)->f32 {
    // Do not add a small inverse displacement to a large cache coordinate.
    // Reduce the fixed phase first so iteration retains its own precision.
    let base=dot(k,point)+phase;
    return base-floor(base/6.283185307179586)*6.283185307179586+dot(k,offset);
}
fn sample_long(point: vec2<f32>, max_k: f32) -> Field {
    var offset=vec2(0.0);
    // With the admitted contraction below 0.9 the residual shrinks by at
    // least 0.9 each iteration. Normal sea samples exit much earlier; the
    // larger cap also converges near a strongly compressed regular crest.
    for(var iteration=0u; iteration<256u; iteration++) {
        var next=vec2(0.0);
        for(var j=0u; j<p.time.y; j++) {
            let c=components[j]; let k=c.physical.xy; let magnitude=length(k);
            if magnitude<=max_k {
                let angle=long_angle(k,point,offset,c.physical.w);
                next+=p.physical.x*c.physical.z*k/magnitude*sin(angle);
            }
        }
        let error=length(next-offset); offset=next;
        // A sub-ULP fixed-point oscillation cannot improve the sample. The
        // 10 micrometre residual stays inside the height/slope oracle bounds,
        // including the near-0.9 compressed-crest regression.
        if error<0.00001+0.0000005*length(offset) { break; }
    }
    var h=0.0; var gradient=vec2(0.0); var differential=vec3(0.0); var velocity=vec3(0.0);
    for(var j=0u; j<p.time.y; j++) {
        let c=components[j]; let k=c.physical.xy; let magnitude=length(k);
        if magnitude<=max_k {
            let angle=long_angle(k,point,offset,c.physical.w); let s=sin(angle); let co=cos(angle);
            let amp=c.physical.z; let omega=c.rate.x;
            h+=amp*co; gradient-=amp*k*s;
            differential-=p.physical.x*amp/magnitude*vec3(k.x*k.x,k.x*k.y,k.y*k.y)*co;
            velocity+=amp*omega*vec3(p.physical.x*k.x/magnitude*co,s,p.physical.x*k.y/magnitude*co);
        }
    }
    let jac=(1.0+differential.x)*(1.0+differential.z)-differential.y*differential.y;
    let slope=vec2(gradient.x*(1.0+differential.z)-gradient.y*differential.y,
        gradient.y*(1.0+differential.x)-gradient.x*differential.y)/jac;
    return Field(vec4(-offset.x,h,-offset.y,jac),vec4(slope,0.0,0.0),
        vec4(velocity,velocity.y-dot(slope,velocity.xz)));
}
