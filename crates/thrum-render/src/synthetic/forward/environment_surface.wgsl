    let diffuse=ao*(1.0-metalness)*base*(vec3(1.0)-ess)*ambient_irradiance(n,world_position,pixel_position)/3.14159265359;
    // Single-scatter GGX energy loss is retained; aerial and pre-exposure are
    // applied once by the caller after direct, indirect and emissive combine.
