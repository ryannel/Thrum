fn cloud_bilinear(field:texture_2d_array<f32>,coordinate:vec2<f32>,level:i32)->vec4<f32> {
    let size=vec2<i32>(textureDimensions(field));let p=coordinate-0.5;let lo=vec2<i32>(floor(p));let f=fract(p);
    return mix(mix(textureLoad(field,clamp(lo,vec2(0),size-1),level,0),textureLoad(field,clamp(lo+vec2(1,0),vec2(0),size-1),level,0),f.x),mix(textureLoad(field,clamp(lo+vec2(0,1),vec2(0),size-1),level,0),textureLoad(field,clamp(lo+vec2(1),vec2(0),size-1),level,0),f.x),f.y);
}
// Integral of the cubic edge ramp from its clear end to height h.
fn cloud_fade_integral(h:f32,width:f32)->f32 {
    let t=clamp(h/width,0.0,1.0);
    return width*t*t*t*(1.0-0.5*t)+max(0.0,h-width);
}
// Density, above optical depth, below optical depth, and height through the column.
fn cloud_column(point:vec3<f32>)->vec4<f32> {
    var result=vec4(0.0);
    var remaining=1.0;
    // A fully covering fine window replaces every coarser value.
    // Read coarse windows only for the uncovered fraction of the overlap.
    for(var level=0;level<4;level++) {
        let m=cloud.weather[level];let uv=(point.xz-m.xy)/m.z;
        let edge=min(uv,vec2(m.w)-uv);let w=cloud_smooth(min(edge.x,edge.y)/(m.w*0.15));
        if w==0.0 {continue;}
        let c=cloud_bilinear(columns,uv,level);
        let thickness=c.y-c.x;var value=vec4(0.0);
        if thickness>0.0 && point.y>c.x && point.y<c.y {
            let o=cloud_bilinear(optical,uv,level);
            let h=(point.y-c.x)/thickness;
            let vertical=cloud_smooth(h/o.z)*cloud_smooth((1.0-h)/o.w);
            let integral=1.0-(o.z+o.w)*0.5;
            let above=clamp((cloud_fade_integral(1.0-h,o.w)-max(0.0,0.5*o.z-cloud_fade_integral(h,o.z)))/integral,0.0,1.0);
            // Interpolate the profile itself. Interpolating the integral and
            // dividing by thickness again would keep the cloudy texel's full
            // profile as its neighbour thins continuously toward clear air.
            let peak=cloud.physical.x*c.z;
            let tau=peak*thickness*integral;
            value=vec4(peak*vertical,tau*above,tau*(1.0-above),h);
        }
        result+=value*(remaining*w);
        if w==1.0 {break;}
        remaining*=1.0-w;
    }
    return result;
}
