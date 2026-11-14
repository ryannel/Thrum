// Shared body-query/cache evaluator: invert horizontal displacement at the world point.
fn sample_long(point: vec2<f32>, max_k: f32) -> Field {
    var offset=vec2(0.0);
    for(var iteration=0u; iteration<24u; iteration++) {
        var next=vec2(0.0);
        for(var j=0u; j<p.time.y; j++) {
            let c=components[j]; let k=c.physical.xy; let magnitude=length(k);
            if magnitude<=max_k {
                let angle=dot(k,point+offset)+c.physical.w;
                next+=p.physical.x*c.physical.z*k/magnitude*sin(angle);
            }
        }
        let error=length(next-offset); offset=next;
        if error<0.000001 { break; }
    }
    var h=0.0; var gradient=vec2(0.0); var differential=vec3(0.0); var velocity=vec3(0.0);
    for(var j=0u; j<p.time.y; j++) {
        let c=components[j]; let k=c.physical.xy; let magnitude=length(k);
        if magnitude<=max_k {
            let angle=dot(k,point+offset)+c.physical.w; let s=sin(angle); let co=cos(angle);
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
