@group(3) @binding(0) var weather_columns:texture_2d_array<f32>;
@group(3) @binding(1) var weather_optical:texture_2d_array<f32>;
@group(3) @binding(2) var placed_columns:texture_2d_array<f32>;
@group(3) @binding(3) var ground_sun_path:texture_2d<f32>;

fn field_colour(field:u32,uv:vec2<f32>)->vec4<f32> {
    let weather_pixel=vec2<i32>(uv*vec2<f32>(textureDimensions(weather_columns)));
    let column=textureLoad(weather_columns,weather_pixel,0,0);
    var value=0.0;
    switch field {
        case 1u: { value=column.z; }
        case 2u: { value=column.w; }
        case 3u: { value=column.y/max(cloud.heights.y,1.0); }
        case 4u: {
            let age=textureLoad(weather_optical,weather_pixel,0,0).x;
            value=select(0.0,fract(age/120.0),age>0.0);
        }
        case 5u: {
            if cloud.density_profile.y>0.0 {
                let placed=textureLoad(placed_columns,weather_pixel,0,0);
                if placed.x>0.0 {
                    return vec4(0.30+0.70*fract(placed.x*vec3(0.618,0.414,0.732)),0.0);
                }
            }
        }
        case 6u: {
            let pixel=vec2<i32>(uv*vec2<f32>(textureDimensions(ground_sun_path)));
            value=1.0-exp(-textureLoad(ground_sun_path,pixel,0).x);
        }
        case 7u: {
            let pixel=vec2<i32>(uv*vec2<f32>(cloud.active_extent));
            value=textureLoad(front,pixel,0).w/max(1.0,f32(cloud.temporal.z*cloud.temporal.z-1u));
            if cloud.temporal.w!=0u { value=0.0; }
        }
        default: {}
    }
    let heat=mix(vec3(0.012,0.025,0.085),vec3(1.0,0.72,0.12),clamp(value,0.0,1.0));
    return vec4(heat,0.0);
}
