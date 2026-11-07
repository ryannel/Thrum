// Game material hook glue, compiled only into a hooked cooked mesh pipeline: after the scene's
// forward shader and the cooked entries, before the game's hook. The scene's shade() reads its
// material record through cooked_hooked_material, which hands the hook the record's surface
// factors and copies back only factors. Flags, texture layers and the alpha cutoff are kept.

// Surface factors a hook may change. The base colour multiplies the cooked base-colour texture;
// emission is radiance in the scene's units, added to the lit surface.
struct CookedSurface {
    base_colour: vec4<f32>,
    roughness: f32,
    metalness: f32,
    emissive: vec3<f32>,
}

// Where the fragment is shaded: texture coordinates, world position (metres) and unit normal.
struct CookedSurfacePoint {
    uv: vec2<f32>,
    world: vec3<f32>,
    normal: vec3<f32>,
}

fn cooked_hooked_material(record: MaterialRecord, input: VertexOutput) -> MaterialRecord {
    let point = CookedSurfacePoint(input.uv, input.world_position, normalize(input.normal));
    let given = CookedSurface(record.base, record.surface.y, record.surface.z, record.emissive_cutoff.rgb);
    let hooked = cooked_surface(given, point);
    var result = record;
    result.base = clamp(hooked.base_colour, vec4(0.0), vec4(1.0));
    result.surface.y = clamp(hooked.roughness, 0.0, 1.0);
    result.surface.z = clamp(hooked.metalness, 0.0, 1.0);
    result.emissive_cutoff = vec4(max(hooked.emissive, vec3(0.0)), record.emissive_cutoff.w);
    return result;
}

// The game's hook follows:
