@group(0) @binding(0) var image:texture_2d<f32>;
@group(0) @binding(1) var samp:sampler;
@group(0) @binding(2) var<uniform> params:array<vec4f,10>;
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
  let a=array<vec2f,3>(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));
  return vec4f(a[i],0,1);
}
@fragment fn fs(@builtin(position) p:vec4f)->@location(0) vec4f {
  let uv=clamp(p.xy/vec2f(1280,720)*params[8].xy,vec2f(.5),params[8].xy-vec2f(.5))/vec2f(textureDimensions(image));
  var c=textureSampleLevel(image,samp,uv,0.).rgb;
  let a=2.51;
  let b=.03;
  let d=2.43;
  let e=.59;
  let f=.14;
  c=clamp((c*(a*c+b))/(c*(d*c+e)+f),vec3f(0),vec3f(1));
  return vec4f(pow(c,vec3f(1./2.2)),1);
}
