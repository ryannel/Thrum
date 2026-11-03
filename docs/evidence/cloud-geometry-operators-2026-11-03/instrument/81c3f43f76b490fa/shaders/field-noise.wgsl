fn hash3(x:vec3f)->f32 {
  return fract(sin(dot(x,vec3f(127.1,311.7,74.7)))*43758.5453);
}
fn valueNoise(x:vec3f)->f32 {
  let i=floor(x);
  let f=fract(x);
  let u=f*f*(3.-2.*f);
  return mix(mix(mix(hash3(i),hash3(i+vec3f(1,0,0)),u.x),mix(hash3(i+vec3f(0,1,0)),hash3(i+vec3f(1,1,0)),u.x),u.y),mix(mix(hash3(i+vec3f(0,0,1)),hash3(i+vec3f(1,0,1)),u.x),mix(hash3(i+vec3f(0,1,1)),hash3(i+vec3f(1,1,1)),u.x),u.y),u.z);
}
// Prepared field data; one lookup, with no per-voxel hash/sine work.
fn preparedFieldNoise(field:texture_3d<f32>,repeat:sampler,coord:vec3f,lod:f32)->f32 {
  return clamp(textureSampleLevel(field,repeat,fract(coord),lod).x,0.,1.);
}
