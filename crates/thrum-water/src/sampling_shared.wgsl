fn sample_scaled_field(t:texture_2d_array<f32>,s:sampler,uv:vec2<f32>,layer:i32,dx:vec2<f32>,dy:vec2<f32>,footprint_scale:f32)->vec4<f32> {
    let wind = textureNumLayers(t) == RAIN_SHARED_LAYERS && layer < i32(RAIN_FIRST_LAYER);
    let extent=vec2<f32>(textureDimensions(t, select(0, 1, wind)));
    let x=dx*extent;let y=dy*extent;
    let trace=dot(x,x)+dot(y,y);let determinant=x.x*y.y-x.y*y.x;
    let major2=0.5*(trace+sqrt(max(0.0,trace*trace-4.0*determinant*determinant)));
    let minor=sqrt(determinant*determinant/max(major2,0.00000001));
    // Once even the short side covers a texel, retain the ordinary hardware
    // ellipse unchanged. This never picks a mip from the footprint's long side.
    let reconstruction=1.0-smoothstep(0.5,1.0,minor*footprint_scale);
    if reconstruction==0.0 {return textureSampleGrad(t,s,uv,layer,dx,dy);}
    let p=uv*extent-0.5;let base=floor(p);let f=fract(p);
    let wx=cubic_weights(f.x);let wy=cubic_weights(f.y);
    let gx=vec2(wx.x+wx.y,wx.z+wx.w);let gy=vec2(wy.x+wy.y,wy.z+wy.w);
    let hx=vec2(-1.0+wx.y/gx.x,1.0+wx.w/gx.y);
    let hy=vec2(-1.0+wy.y/gy.x,1.0+wy.w/gy.y);
    let a=(base+vec2(hx.x,hy.x)+0.5)/extent;
    let b=(base+vec2(hx.y,hy.x)+0.5)/extent;
    let c=(base+vec2(hx.x,hy.y)+0.5)/extent;
    let d=(base+vec2(hx.y,hy.y)+0.5)/extent;
    let cubic=gy.x*(gx.x*textureSampleGrad(t,s,a,layer,dx,dy)+gx.y*textureSampleGrad(t,s,b,layer,dx,dy))
        +gy.y*(gx.x*textureSampleGrad(t,s,c,layer,dx,dy)+gx.y*textureSampleGrad(t,s,d,layer,dx,dy));
    if reconstruction==1.0 {return cubic;}
    return mix(textureSampleGrad(t,s,uv,layer,dx,dy),cubic,reconstruction);
}

fn sample_field(t:texture_2d_array<f32>,s:sampler,uv:vec2<f32>,layer:i32,dx:vec2<f32>,dy:vec2<f32>)->vec4<f32> {
    return sample_scaled_field(t,s,uv,layer,dx,dy,1.0);
}
