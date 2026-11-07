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

// Exponent bits all set means infinity or NaN. Testing bits rather than comparing floats keeps
// the check under fast-math compilers, which may assume NaN never occurs.
const COOKED_GLUE_EXPONENT: u32 = 0x7f800000u;
// The largest finite half-float, the scene target's format: brighter emission cannot be stored.
const COOKED_GLUE_EMISSION_LIMIT: f32 = 65504.0;

fn cooked_glue_finite(value: f32, fallback: f32) -> f32 {
    return select(fallback, value, (bitcast<u32>(value) & COOKED_GLUE_EXPONENT) != COOKED_GLUE_EXPONENT);
}

fn cooked_glue_finite3(value: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let exponent = bitcast<vec3<u32>>(value) & vec3(COOKED_GLUE_EXPONENT);
    return select(fallback, value, exponent != vec3(COOKED_GLUE_EXPONENT));
}

fn cooked_glue_finite4(value: vec4<f32>, fallback: vec4<f32>) -> vec4<f32> {
    let exponent = bitcast<vec4<u32>>(value) & vec4(COOKED_GLUE_EXPONENT);
    return select(fallback, value, exponent != vec4(COOKED_GLUE_EXPONENT));
}

// A factor the hook made infinite or NaN falls back to the placement's own value; the rest are
// clamped to their valid ranges, and emission to what the scene target can hold.
fn cooked_hooked_material(record: MaterialRecord, input: VertexOutput) -> MaterialRecord {
    let point = CookedSurfacePoint(input.uv, input.world_position, normalize(input.normal));
    let given = CookedSurface(record.base, record.surface.y, record.surface.z, record.emissive_cutoff.rgb);
    let hooked = cooked_surface(given, point);
    var result = record;
    result.base = clamp(cooked_glue_finite4(hooked.base_colour, given.base_colour), vec4(0.0), vec4(1.0));
    result.surface.y = clamp(cooked_glue_finite(hooked.roughness, given.roughness), 0.0, 1.0);
    result.surface.z = clamp(cooked_glue_finite(hooked.metalness, given.metalness), 0.0, 1.0);
    let emissive = cooked_glue_finite3(hooked.emissive, given.emissive);
    result.emissive_cutoff = vec4(clamp(emissive, vec3(0.0), vec3(COOKED_GLUE_EMISSION_LIMIT)), record.emissive_cutoff.w);
    return result;
}

// The game's hook follows:
