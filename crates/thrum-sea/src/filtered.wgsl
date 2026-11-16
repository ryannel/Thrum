// Five linear planes per wave layer: displacement/J, derivatives, velocity,
// slope second moments/dZdz, and J squared. Mips average all planes unchanged.
@group(0) @binding(0) var<storage,read> cascades:array<Field>;
@group(0) @binding(1) var<storage,read> long_field:array<Field>;
@group(0) @binding(2) var cascade_out:texture_storage_2d_array<rgba16float,write>;
@group(0) @binding(3) var long_out:texture_storage_2d_array<rgba16float,write>;
fn plane(f:Field,c:u32)->vec4<f32> {
    switch c {
        case 0u:{return f.displacement;}
        case 1u:{return f.slope;}
        case 2u:{return f.velocity;}
        case 3u:{return vec4(f.slope.x*f.slope.x,f.slope.x*f.slope.y,f.slope.y*f.slope.y,f.velocity.w);}
        default:{return vec4(f.displacement.w*f.displacement.w,0.0,0.0,0.0);}
    }
}
@compute @workgroup_size(8,8)
fn pack(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.z<15u && all(id.xy<vec2(256u)) {
        let f=cascades[(id.z/5u)*65536u+id.y*256u+id.x];
        textureStore(cascade_out,vec2<i32>(id.xy),i32(id.z),plane(f,id.z%5u));
    }
    if id.z<40u && all(id.xy<vec2(128u)) {
        let f=long_field[4096u+(id.z/5u)*16384u+id.y*128u+id.x];
        textureStore(long_out,vec2<i32>(id.xy),i32(id.z),plane(f,id.z%5u));
    }
}
@group(0) @binding(4) var parent:texture_2d_array<f32>;
@group(0) @binding(5) var child:texture_storage_2d_array<rgba16float,write>;
@compute @workgroup_size(8,8)
fn reduce_texture(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=textureDimensions(child)) || id.z>=textureNumLayers(child) {return;}
    let q=vec2<i32>(id.xy*2u);let layer=i32(id.z);
    let value=(textureLoad(parent,q,layer,0)+textureLoad(parent,q+vec2(1,0),layer,0)
        +textureLoad(parent,q+vec2(0,1),layer,0)+textureLoad(parent,q+vec2(1,1),layer,0))*0.25;
    textureStore(child,vec2<i32>(id.xy),layer,value);
}
