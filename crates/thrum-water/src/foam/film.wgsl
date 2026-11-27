    // The youngest film froths in a continuous front before the holes in
    // the pattern resolve. The same stock then opens out into the wash.
    var front = 0.0;
    if SURF_CACHE_ENABLED && face > 0.0 {
        let churned_front = 0.65 * foam_noise(point.xz * 1.7) + 0.35 * foam_noise(point.xz * 5.3 + 29.0);
        front = smoothstep(0.0, 0.70, face) * smoothstep(0.01, 0.14, fresh_front) *
            mix(0.42, 0.88, smoothstep(0.18, 0.83, churned_front));
    }
    return vec4(air, max(lace, front), stocks.zw);
}

@group(1) @binding(0) var shore_depth: texture_depth_2d;

// The film needs only the sign of the same wave column the query uses.
// A fixed-point residual bounds the error in rest position and therefore
// in height. Stop as soon as that interval lies entirely on one side of bed.
fn shore_is_wet(point: vec2<f32>, gains: vec4<f32>, depth: f32) -> bool {
    var thickness = depth + wake_at(point, 0.0).x;
    for (var band = 0u; band < 3u; band++) {
        let gain = gains[band + 1u];
        if gain * water.heights[band + 1u] <= 0.0 { continue; }
        let uv = point / water.lengths[band];
        let layer = i32(band * 5u);
        thickness += gain * (periodic_texel(uv, layer, 0).y + periodic_texel(uv, layer + 4, 0).z);
    }
    let gain = gains.x;
    if gain == 0.0 || abs(thickness) > gain * foam_settings.long_bounds.x + 0.0001 {
        return thickness > 0.0;
    }
    let chop = foam_settings.camera.w;
    let slope = gain * foam_settings.long_bounds.y;
    // Water preparation admits a contraction below 0.9 before depth limiting.
    let contraction = min(0.9 * gain, chop * slope);
    let p = point - foam_settings.camera.xy;
    var offset = vec2(0.0);
    var height = thickness;
    for (var iteration = 0u; iteration < 192u; iteration++) {
        var next = vec2(0.0);
        height = thickness;
        for (var i = 0u; i < foam_settings.banks.w; i++) {
            let c = column_component(i);
            let phase = column_phase(c, p, offset, 0.0);
            let amplitude = gain * c.amplitude;
            height += amplitude * cos(phase);
            next += (amplitude * chop * c.inverse_k * sin(phase)) * c.vector;
        }
        let residual = length(next - offset);
        if abs(height) > (slope / (1.0 - contraction)) * residual + 0.0001 {
            return height > 0.0;
        }
        if residual < 0.00001 { break; }
        offset = next;
    }
    return height > 0.0;
}

@vertex fn film_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment fn film_fragment(@builtin(position) pixel: vec4<f32>) -> Output {
    let uv = pixel.xy * view_aux.viewport_inv_viewport.zw;
    let depth = textureLoad(shore_depth, vec2<i32>(pixel.xy), 0);
    if depth < 1e-7 { discard; }
    let clip = vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth, 1.0);
    let homogeneous = view_aux.inverse_view_proj * clip;
    let ground = homogeneous.xyz / max(homogeneous.w, 1e-9);
    let footprint = length(ground - view.camera_xyz_time.xyz) * view_aux.viewport_inv_viewport.w;
    let answer = map_sample(ground.xz);
    if answer.body == NO_BODY || !answer.has_bed { discard; }
    let row = surface_row(ground.xz, map_cell(ground.xz), answer.row, answer.body, true);
    let reach = 0.175 * row.swash;
    let above = row.bed - row.level;
    let seaward_fade = -0.46 * row.swash;
    if reach <= 0.0 || above >= reach || above <= seaward_fade { discard; }
    // The depth already on screen must belong to the ground. A hull, a hill
    // in front of the beach, or the visible sheet above the bed masks the film.
    if abs(ground.y - row.bed) > 0.06 { discard; }
    // Surf draws its displaced sheet over the beach film in the next pass.
    // Only the plain surface uses the iteration to mask the film in advance.
    if !SURF_CACHE_ENABLED && shore_is_wet(ground.xz, row.gains, -above) { discard; }
    // The ground's optical weight depends on elevation. Stranded foam is
    // patterned separately so a dense stock cannot paint a solid white sheet.
    let sheen = (1.0 - smoothstep(0.0, reach, above)) * smoothstep(seaward_fade, -0.10 * row.swash, above);
    let stocks = foam_at(ground.xz, vec4(0.0)).stocks;
    let openings = foam_noise(ground.xz * vec2(0.87, 1.53));
    let porous = mix(0.10, 0.60, smoothstep(0.22, 0.76, openings));
    let stranded = vec4(stocks.x, stocks.yz * vec2(0.54, 1.0) * porous, stocks.w);
    let foam = ranked_cover(stranded, ground.xz, footprint) * sheen;
    let v = normalize(view.camera_xyz_time.xyz - ground);
    let ray = reflect(-v, vec3(0.0, 1.0, 0.0));
    let sky = environment_sky_radiance(ray);
    let fresnel = water_fresnel(v.y);
    let incident = environment_diffuse(vec3(0.0, 1.0, 0.0)) + water_sun(ground) * max(0.0, environment_sun_direction().y);
    let reflection = sheen * fresnel * 0.72 * (1.0 - foam);
    let opacity = reflection + foam;
    let light = (reflection * sky + foam * 0.24 * incident) / max(1e-6, opacity);
    let air = environment_aerial(ground, uv);
    let colour = (light * air.transmission + air.scatter) * environment_storage_scale();
    return Output(vec4(colour, opacity), vec2(0.0), foam);
}
