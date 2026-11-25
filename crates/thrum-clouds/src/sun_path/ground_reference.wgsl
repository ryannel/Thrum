// Ground-only replay. The production shader packs cumulative nodes; this
// kernel integrates the complete ray directly and writes one f32 per column.
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var columns: texture_2d_array<f32>;
@group(0) @binding(2) var optical: texture_2d_array<f32>;
@group(0) @binding(3) var expected: texture_storage_2d<r32float, write>;
@group(0) @binding(4) var<storage, read> current_rows: array<u32>;
@group(0) @binding(9) var shape: texture_3d<f32>;
@group(0) @binding(10) var shape_sampler: sampler;

fn extinction_at(point: vec3<f32>) -> f32 {
    var column = vec2(0.0);
    for (var level = 3; level >= 0; level--) {
        let map = p.weather[level];
        let uv = (point.xz - map.xy) / map.z;
        let inside = min(uv, vec2<f32>(map.w) - uv);
        let weight = smoothstep(0.0, 0.15 * map.w, min(inside.x, inside.y));
        if weight == 0.0 { continue; }

        let cell = uv - 0.5;
        let base = vec2<i32>(floor(cell));
        let fraction = fract(cell);
        let limit = vec2<i32>(i32(map.w) - 1);
        var c = vec4<f32>(0.0);
        var o = vec4<f32>(0.0);
        for (var corner = 0u; corner < 4u; corner++) {
            let offset = vec2<u32>(corner % 2u, corner / 2u);
            let at = clamp(base + vec2<i32>(offset), vec2<i32>(0), limit);
            let axis = select(1.0 - fraction, fraction, offset == vec2<u32>(1));
            let tap = axis.x * axis.y;
            c += tap * textureLoad(columns, at, level, 0);
            o += tap * textureLoad(optical, at, level, 0);
        }
        var local = vec2(0.0);
        if c.x < point.y && point.y < c.y {
            let h = (point.y - c.x) / (c.y - c.x);
            let fades = smoothstep(0.0, o.z, h) * (1.0 - smoothstep(1.0 - o.w, 1.0, h));
            local = vec2(p.physical.z * c.z * fades, h);
        }
        column = mix(column, local, weight);
    }
    let density = column.x;
    if density == 0.0 || p.physical.w == 0.0 { return density; }
    let uv = point / 6400.0 + vec3(p.erosion.y, 0.0, p.erosion.z);
    let billow = textureSampleLevel(shape, shape_sampler, uv, erosion_mip(p.erosion.w, 50.0)).r;
    let eroded = cloud_erode(density, p.physical.z, p.physical.w, billow, p.erosion.x);
    if eroded == 0.0 { return 0.0; }
    return eroded * cloud_bank_shape(shape, shape_sampler, point, uv, p.bank_noise,
                                    p.erosion.w, column.y, cloud_bank_weight(p.bank_noise.w));
}

@compute @workgroup_size(8, 8)
fn replay(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(u32(p.origin.w))) || current_rows[id.y] == 0u { return; }
    let column = (vec2<f32>(id.xy) + 0.5) * p.origin.z;
    let bottom = p.physical.x;
    let top = p.physical.y;
    let spacing = (top - bottom) / f32(p.rows.z - 1u);
    let subdivisions = u32(ceil(spacing / (40.0 * p.direction.y)));
    let step = spacing / f32(subdivisions);
    let count = (p.rows.z - 1u) * subdivisions;
    var integral = 0.0;
    for (var i = count; i > 0u; i--) {
        let sample_index = i - 1u;
        let offset = f32(sample_index / subdivisions) * spacing
            + (f32(sample_index % subdivisions) + 0.5) * step;
        let height = top - offset;
        let point = column + p.direction.xz * (height - bottom) / p.direction.y;
        integral += extinction_at(vec3(point.x, height, point.y));
    }
    textureStore(expected, vec2<i32>(id.xy), vec4(min(65504.0, integral * step / p.direction.y)));
}
