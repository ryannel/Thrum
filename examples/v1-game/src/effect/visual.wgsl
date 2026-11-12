struct Params { work:vec4<u32>, slicing:vec4<u32>, origin:vec4<f32>, pad:vec4<u32>, view:mat4x4<f32> };
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var trail:texture_2d<u32>;
@group(0) @binding(2) var derived:texture_2d<u32>;
struct Vertex { @builtin(position) clip:vec4<f32>, @location(0) world:vec2<f32> };
@vertex fn vertex(@builtin(vertex_index) i:u32)->Vertex {
    let corners=array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(0.,1.),vec2(0.,1.),vec2(1.,0.),vec2(1.,1.));
    let q=(corners[i]-vec2(.5))*256.;
    var out:Vertex;out.clip=p.view*vec4(q.x,p.origin.y,q.y,1.);out.world=q;return out;
}
// Diagnostic motion is deliberately invalid until the M3 reconstruction-input proof.
struct Output { @location(0) color:vec4<f32>, @location(1) motion:vec2<f32>, @location(2) reactive:f32 };
@fragment fn fragment(input:Vertex)->Output {
    let texel=((vec2<i32>(floor(input.world))%vec2<i32>(32))+vec2<i32>(32))%vec2<i32>(32);
    let value=(.85*f32(textureLoad(trail,texel,0).x)+.05*f32(textureLoad(derived,texel,0).x))/255.;
    let emitter=select(0.,1.,p.work.w!=0u && texel.x==i32((p.work.x/3u)%32u) && texel.y==16);
    let checker=f32((i32(floor(input.world.x/4.))+i32(floor(input.world.y/4.)))&1);
    var out:Output;out.color=vec4((vec3(2000.,2200.,2400.)*(.8+.2*checker)+vec3(3000.,36000.,18000.)*value+vec3(60000.,20000.,0.)*emitter)/1024.,1.);out.motion=vec2(65504.);out.reactive=1.;return out;
}
