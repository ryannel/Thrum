@group(3) @binding(0) var<uniform> sheet_clock: vec4<f32>;

struct SheetVertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_time: vec4<f32>,
    @location(1) velocity_along: vec4<f32>,
    @location(2) discharge: vec4<f32>,
    @location(3) across_fraction: vec4<f32>,
    @location(4) edge: f32,
    @location(5) receiving: vec3<f32>,
    @location(6) foam: vec2<f32>,
    @location(7) strand_motion: vec3<f32>,
    @location(8) @interpolate(flat) closed_strand: u32,
    @location(9) strand_normal: vec3<f32>,
    @location(10) @interpolate(flat) strand_lip: f32,
    @location(11) @interpolate(flat) strand_strength: f32,
}

fn sheet_age(age: f32) -> f32 { return age - floor(age / FALL_LOOP) * FALL_LOOP; }

fn aeration_packet(lip: f32, birth: f32) -> f32 {
    // Emit irregular packets of fixed duration at each strand's departure
    // point. The stored flight time stretches their image on faster water.
    let cells = max(1.0, floor(FALL_LOOP * 3.8));
    let seed = lip * 59.3;
    let emitted = birth * (cells / FALL_LOOP) + noise_hash(vec2(seed, 37.0));
    let cell = floor(emitted);
    let slot = cell - floor(cell / cells) * cells;
    let centre = mix(0.28, 0.72, noise_hash(vec2(seed, slot)));
    let radius = mix(0.17, 0.26, noise_hash(vec2(seed + 51.0, slot)));
    return 1.0 - smoothstep(0.0, radius, abs(fract(emitted) - centre));
}

fn fold_weight(fraction: f32) -> f32 {
    let departure = smoothstep(0.12, 0.47, fraction);
    let contact = 1.0 - smoothstep(0.77, 1.0, fraction);
    return departure * contact;
}

fn sheet_fold(lip: f32, birth: f32, across: vec2<f32>) -> vec3<f32> {
    let parcel = sheet_noise(vec2(lip, birth), FALL_LOOP, vec2(1.15, 1.75), vec2(8.4, 0.0));
    let fold = 0.24 * (parcel * 2.0 - 1.0);
    return fold * vec3(-across.y, 0.0, across.x);
}

// Thinner sheets separate below the tongue before breaking into filaments.
fn strand_breakup(thickness: f32) -> f32 {
    return 1.0 - smoothstep(1.75, 2.48, thickness / FALL_TEAR);
}

fn strand_offset(parcel: f32, lane: f32, fraction: f32, width_offset: f32,
    width_axis: vec3<f32>, outward: vec3<f32>) -> vec3<f32> {
    let opening = smoothstep(0.22 + 0.32 * lane, 0.76, fraction);
    let width = mix(0.28, 0.93, 0.59 * lane + 0.41 * parcel);
    let horizontal = width_axis * (0.11 * (parcel - 0.5) - width * width_offset);
    // Let adjacent strands use different depths once they leave the tongue.
    // The offset remains small where their contacts enter the plunge.
    let spread = mix(0.02, 0.31, lane * lane) + 0.11 * (parcel - 0.5);
    let depth = outward * spread * (1.0 - 0.65 * smoothstep(0.70, 1.0, fraction));
    return opening * (horizontal + depth);
}

fn splash_vertex(world_time: vec4<f32>, velocity_along: vec4<f32>, discharge: vec4<f32>,
    across_fraction: vec4<f32>) -> SheetVertex {
    let seed = vec2(velocity_along.w * 23.71, discharge.z);
    let cycles = floor(FALL_LOOP * mix(2.1, 4.4, noise_hash(seed + vec2(7.0))));
    let frequency = cycles / FALL_LOOP;
    let clock = (sheet_clock.x - world_time.w) * frequency + noise_hash(seed + vec2(3.0));
    let cycle = floor(clock) - floor(clock / cycles) * cycles;
    let phase = fract(clock);
    let random = noise_hash(seed + vec2(17.0, cycle));
    let kick = noise_hash(seed + vec2(cycle, 32.0));
    let impact = discharge.z < FALL_CONTACT_DROPS;
    let axis = across_fraction.xz;
    let across = vec3(axis.x, 0.0, axis.y);
    let face = vec3(-axis.y, 0.0, axis.x);
    let flow = velocity_along.xyz * vec3(1.0, 0.0, 1.0);
    let outward = select(-face, face, dot(face, flow) >= 0.0);
    let source = world_time.xyz + outward * select(0.0, 0.14, impact);
    let foam = fall_foam(source.xz);
    let strength = max(smoothstep(0.005, 0.047, discharge.w),
        select(0.0, 0.72 * smoothstep(0.08, 0.48, foam.x + foam.y), impact));
    let drift = outward * mix(0.13, 0.59, kick) * select(1.0, 0.56, impact)
        + across * (random - 0.5) * 0.54;
    let spray = smoothstep(0.009, 0.018, discharge.w) * strand_breakup(discharge.w);
    let loft = (select(mix(0.034, 0.39, kick * kick), mix(0.037, 0.13, kick), impact)
        + select(0.0, spray * mix(0.07, 0.24, kick), impact))
        * mix(0.30, 1.0, strength);
    let pool_height = clamp(wake_at(source.xz, 0.0).x, -0.42, 0.42);
    let centre = source + drift * phase + vec3(0.0, pool_height, 0.0)
        + vec3(0.0, 4.0 * loft * phase * (1.0 - phase), 0.0);
    let eye = normalize(view.camera_xyz_time.xyz - centre);
    let climb = drift + vec3(0.0, 4.0 * loft * (1.0 - 2.0 * phase), 0.0);
    let right = normalize(cross(eye, climb) + vec3(1e-6, 0.0, 0.0));
    let up = cross(right, eye);
    let index = u32(across_fraction.w);
    let corner = vec2(f32(index & 1u), f32(index >> 1u)) * 2.0 - 1.0;
    let radius = (select(mix(0.018, 0.089, random * random), mix(0.11, 0.23, random), impact)
        + select(0.0, spray * mix(0.08, 0.19, kick), impact))
        * mix(0.45, 1.0, strength);
    let point = centre + radius * (right * corner.x + up * corner.y * (1.0 + 1.3 * kick));
    let world = vec4(point, phase);
    return SheetVertex(view.view_proj * vec4(point, 1.0), world, velocity_along,
        vec4(random, kick, strength, select(0.0, 1.0, impact)),
        vec4(corner.x, -1.0, corner.y, phase), 0.0, vec3(0.0), foam, vec3(0.0), 0u, vec3(0.0), 0.0, 0.0);
}

fn strand_swelling(birth: f32, velocity_along: vec4<f32>, across_fraction: vec4<f32>,
    side: f32) -> vec3<f32> {
    let across = vec3(across_fraction.x, 0.0, across_fraction.z);
    let normal = normalize(cross(across, velocity_along.xyz) + vec3(1e-6, 0.0, 0.0));
    let forward = vec3(velocity_along.x, 0.10, velocity_along.z);
    let outward = select(-normal, normal, dot(normal, forward) >= 0.0);
    let angle = side * 1.5707963;
    let radial = across * cos(angle) + outward * sin(angle);
    let pulse = sheet_noise(vec2(velocity_along.w, birth), FALL_LOOP,
        vec2(4.2, 7.3), vec2(47.1, 0.0));
    let fraction = across_fraction.w;
    let fall = smoothstep(0.03, 0.28, fraction) * (1.0 - smoothstep(0.88, 1.0, fraction));
    return radial * fall * (pulse - 0.44) * 0.049;
}

@vertex fn sheet_vertex(
    @location(0) world_time: vec4<f32>,
    @location(1) velocity_along: vec4<f32>,
    @location(2) discharge: vec4<f32>,
    @location(3) across_fraction: vec4<f32>,
    @location(4) edge_receiver: vec4<f32>,
) -> SheetVertex {
    let is_splash = across_fraction.y < 0.0 && across_fraction.y > -1.5;
    if is_splash { return splash_vertex(world_time, velocity_along, discharge, across_fraction); }
    let birth = sheet_age(sheet_clock.x - world_time.w);
    let is_filament = across_fraction.y <= -2.0;
    let closed_strand = edge_receiver.x >= FALL_STRAND_EDGE;
    let wet_tongue = edge_receiver.x >= FALL_TONGUE_EDGE && !closed_strand;
    let axis = vec3(across_fraction.x, 0.0, across_fraction.z);
    let section = max(0.0, edge_receiver.x - FALL_STRAND_EDGE);
    let angle = floor(section) * (6.2831853 / FALL_STRAND_SIDES);
    let strand_strength = select(fract(section) * 2.0, -1.0, wet_tongue);
    let round_edge = select(edge_receiver.x, abs(cos(angle)), closed_strand);
    let edge = select(round_edge, 4.0 * (edge_receiver.x - FALL_TONGUE_EDGE), wet_tongue);
    var strand_normal = vec3(0.0);
    if closed_strand {
        let normal = vec3(axis.z, 0.0, -axis.x);
        strand_normal = axis * cos(angle) + normal * sin(angle);
    }
    let fold_scale = select(select(1.0, 0.24, closed_strand), 0.18, is_filament);
    let weight = fold_weight(across_fraction.w) * fold_scale;
    let fold = sheet_fold(velocity_along.w, birth, across_fraction.xz) * weight;
    var position = world_time.xyz + fold;
    var strand_motion = vec3(0.0);
    if is_filament { position += strand_swelling(birth, velocity_along, across_fraction, edge); }
    let breakup = strand_breakup(discharge.w);
    if closed_strand && breakup > 0.0 && across_fraction.w > 0.21 {
        let parcel = sheet_noise(vec2(edge_receiver.w, birth), FALL_LOOP, vec2(3.5, 3.6), vec2(9.0, 0.0));
        let lane = noise_hash(vec2(edge_receiver.w * 34.63, 18.0));
        let face = vec3(-axis.z, 0.0, axis.x);
        let outward = select(-face, face, dot(face, velocity_along.xyz) >= 0.0);
        let offset = velocity_along.w - edge_receiver.w;
        let last_fraction = across_fraction.w - length(velocity_along.xyz) * sheet_clock.y / across_fraction.y;
        let current = breakup * strand_offset(parcel, lane, across_fraction.w, offset, axis, outward);
        let previous = breakup * strand_offset(parcel, lane, last_fraction, offset, axis, outward);
        position += current;
        strand_motion = current - previous;
    }
    var receiving_motion = 0.0;
    var receiving_level = edge_receiver.y;
    var foam = vec2(0.0);
    if edge_receiver.z > 0.5 && across_fraction.w > 0.63 {
        // Follow the same interaction height as the receiving surface. Only
        // the optical contact moves; source locations stay on the fall record.
        let contact = select(0.0, smoothstep(0.63, 1.0, across_fraction.w), edge_receiver.z > 1.5);
        let height = clamp(wake_at(position.xz, 0.0).x, -0.42, 0.42);
        let previous = clamp(wake_at(position.xz, -sheet_clock.y).x, -0.42, 0.42);
        position.y += contact * height;
        receiving_level += height;
        receiving_motion = contact * (height - previous);
        foam = fall_foam(position.xz);
    }
    let world = vec4(position, world_time.w);
    let strand_lip = select(velocity_along.w, edge_receiver.w, closed_strand);
    return SheetVertex(view.view_proj * vec4(world.xyz, 1.0), world, velocity_along, discharge,
        across_fraction, edge, vec3(receiving_motion, position.y - receiving_level, edge_receiver.z),
        foam, strand_motion, u32(closed_strand), strand_normal, strand_lip, strand_strength);
}

struct FallOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
}

fn splash_shade(input: SheetVertex) -> FallOutput {
    let phase = input.across_fraction.w;
    let corner = input.across_fraction.xz;
    let lobes = 0.82 + 0.18 * noise_hash(floor(corner * 4.0 + input.discharge.xx * 29.0));
    let support = 1.0 - smoothstep(0.06, lobes, dot(corner, corner));
    let alive = smoothstep(0.0, 0.15, phase) * (1.0 - smoothstep(0.69, 1.0, phase));
    let foam_gain = 0.57 + 0.43 * smoothstep(0.04, 0.50, input.foam.x + input.foam.y);
    let alpha = input.discharge.z * support * alive * foam_gain
        * mix(0.32, 0.92, input.discharge.x) * mix(1.0, 0.78, input.discharge.w);
    if alpha < 0.012 { discard; }
    let up = vec3(0.0, 1.0, 0.0);
    let point = input.world_time.xyz;
    let direct = sun_irradiance(point) * fall_cloud(point) * max(0.0, environment_sun_direction().y);
    let foam = (environment_diffuse(up) + direct) * vec3(0.82, 0.89, 0.88) / 3.14159265;
    let air = environment_aerial(point, input.clip.xy * view_aux.viewport_inv_viewport.zw);
    let colour = (foam * air.transmission + air.scatter) * environment_storage_scale();
    return FallOutput(vec4(colour, alpha), vec2(0.0), alpha);
}

fn tongue_shade(input: SheetVertex, look: vec3<f32>) -> FallOutput {
    let tangent = normalize(input.velocity_along.xyz + vec3(1e-7, 0.0, 0.0));
    let across = vec3(input.across_fraction.x, 0.0, input.across_fraction.z);
    let normal = normalize(cross(across, tangent));
    let up = select(-normal, normal, dot(normal, look) >= 0.0);
    let facing = max(0.0, dot(up, look));
    let grazing = pow(1.0 - facing, 3.0);
    let reflection = environment_reflection(reflect(-look, up), 0.18, facing).radiance;
    let diffuse = environment_diffuse(vec3(0.0, 1.0, 0.0));
    let clear = mix(diffuse * 0.18, reflection, 0.42 + 0.30 * grazing);
    let fraction = input.across_fraction.w;
    let wet = smoothstep(-1.0, -0.42, fraction) * (1.0 - smoothstep(0.07, 0.25, fraction));
    let edge = 1.0 - smoothstep(0.57, 1.0, input.edge);
    let alpha = (0.28 + 0.17 * grazing) * wet * edge;
    let air = environment_aerial(input.world_time.xyz, input.clip.xy * view_aux.viewport_inv_viewport.zw);
    let colour = (clear * air.transmission + air.scatter) * environment_storage_scale();
    return FallOutput(vec4(colour, alpha), vec2(0.0), 0.0);
}

fn sheet_shade(input: SheetVertex, solid: bool, front: bool) -> FallOutput {
    let point = input.world_time.xyz;
    let look = normalize(view.camera_xyz_time.xyz - point);
    let lip = input.velocity_along.w;
    let birth = sheet_age(sheet_clock.x - input.world_time.w);
    let down = birth * input.discharge.x;
    let xy = vec2(lip, down) * vec2(2.4, 0.68);
    let span = FALL_LOOP * input.discharge.x * 0.68;
    // Broader parcels leave nearby parts of the lip together. Smaller
    // strands within the parcel stretch at their own speeds.
    let grouped = sheet_noise(vec2(lip * 0.12, birth), FALL_LOOP, vec2(2.7), vec2(0.0));
    let coarse = sheet_noise(xy, span, vec2(2.4), vec2(0.0));
    let fine = sheet_noise(xy, span, vec2(8.7), vec2(0.0));
    let rank = 0.28 * coarse + 0.10 * fine + 0.62 * grouped;
    let pixel = max(length(dpdx(xy)), length(dpdy(xy)));
    let geometry_normal = normalize(cross(dpdx(point), dpdy(point)));
    if input.closed_strand != 0u && !front { discard; }
    if input.across_fraction.y < 0.0 && input.across_fraction.y > -1.5 {
        if solid { discard; }
        return splash_shade(input);
    }
    if input.strand_strength < 0.0 {
        if solid { discard; }
        return tongue_shade(input, look);
    }
    let width = max(0.025, min(0.27, 0.10 * pixel));
    let tangent = normalize(input.velocity_along.xyz + vec3(1e-8, -1e-8, 0.0));
    let across = input.across_fraction.xz;
    let is_filament = input.across_fraction.y <= -2.0;
    let normal = normalize(cross(vec3(across.x, 0.0, across.y), tangent));
    let geometric = select(-geometry_normal, geometry_normal, dot(geometry_normal, normal) > 0.0);
    let fraction = input.across_fraction.w;
    let geometry_weight = select(0.38 * smoothstep(0.34, 0.76, fraction), 1.0, is_filament);
    let sheet_normal = mix(normal, geometric, select(geometry_weight, 0.0, input.closed_strand != 0u));
    // At the root the strand's section is still flat and shares the tongue's normal.
    let rounding_in = select(0.0, smoothstep(FALL_STRAND_ROLL, 0.77, fraction), input.closed_strand != 0u);
    let contour = normalize(mix(sheet_normal, input.strand_normal, rounding_in));
    var face = select(-contour, contour, dot(contour, look) >= 0.0);
    let breakout = input.discharge.y;
    let breakup = select(strand_breakup(input.discharge.w), 0.0, is_filament);
    let below_tongue = smoothstep(0.02, 0.23, fraction);
    let broken = smoothstep(rank - 0.24 - width, rank + 0.09 + width, breakout);
    let parcel = sheet_noise(vec2(lip, birth), FALL_LOOP, vec2(2.1, 5.25), vec2(117.0, 0.0));
    let pulse = smoothstep(0.40, 0.72, 0.86 * parcel + 0.14 * fine);
    let rounded = input.closed_strand != 0u;
    let packet_skew = mix(-0.15, 0.15, noise_hash(vec2(input.strand_lip * 23.3, 79.0)));
    let packet_birth = birth + (lip - input.strand_lip) * packet_skew + (fine - 0.5) * 0.018;
    let packet_core = select(1.0, 1.0 - smoothstep(0.17, 0.93, input.edge), rounded);
    let packet = aeration_packet(input.strand_lip, packet_birth) * packet_core;
    let mixed_air = smoothstep(0.10, 0.33, fraction) * smoothstep(0.0, 0.55, input.strand_strength);
    let radial = normalize(input.strand_normal - tangent * dot(input.strand_normal, tangent)
        + vec3(1e-7, 0.0, 0.0));
    let view_across = normalize(look - tangent * dot(look, tangent) + vec3(1e-7, 0.0, 0.0));
    let chord = smoothstep(0.02, 0.83, abs(dot(radial, view_across)));
    let droplets = sheet_noise(vec2(lip, birth), FALL_LOOP, vec2(42.0, 13.6), vec2(91.0, 0.0));
    // The transparent film dwindles into travelling bursts below the crest.
    // Keep only faint water between bursts so the prepared mesh has no fixed hem.
    let dispersed = smoothstep(0.19, 0.77, fraction);
    let air_breaks = smoothstep(0.29, 0.77, 0.65 * parcel + 0.22 * fine + 0.13 * droplets);
    let wet_pockets = max(packet, air_breaks * (0.42 + 0.58 * chord));
    let middle_cover = mix(1.0, 0.075 + 0.925 * wet_pockets, dispersed * mixed_air);
    let filament_cover = mix(0.09, 1.0, smoothstep(0.05, 0.59, packet + 0.13 * pulse));
    let passing_cover = middle_cover * select(1.0, filament_cover, is_filament);
    let filament_white = smoothstep(0.06, 0.48, input.across_fraction.w)
        * mix(0.22, 1.0, 0.82 * packet + 0.18 * pulse);
    let streak_white = smoothstep(0.42, 0.89, input.across_fraction.w)
        * smoothstep(0.23, 0.78, 0.65 * coarse + 0.35 * fine);
    let parcel_foam = breakup * below_tongue * (0.12 + 0.53 * pulse);
    let base_white = max(mix(pow(breakout, 1.18), broken, 0.74), 0.63 * streak_white);
    let rolled = select(0.0, breakup * smoothstep(0.20, 0.42, fraction), input.closed_strand != 0u);
    let sheet_white = max(max(base_white, parcel_foam), rolled * mix(0.79, 0.99, pulse));
    let white = select(sheet_white * below_tongue, filament_white, is_filament);
    let clear = 1.0 - white;
    // Preserve the long strands of a small discharge when the trail widens
    // in time. The lower-frequency mask follows each parcel from the lip.
    let rivulet = sheet_noise(vec2(lip, birth), FALL_LOOP, vec2(3.2, 0.28), vec2(18.0, 0.0));
    let tear = pow(0.84 * rivulet + 0.16 * coarse, 2.7);
    let threshold = mix(tear, 0.32, smoothstep(1.5, 4.0, pixel));
    let thinness = 1.0 - smoothstep(0.12, 0.45, input.discharge.z);
    let tip = 0.73 + 0.24 * fine;
    let taper = smoothstep(tip - 0.25, tip, input.across_fraction.w);
    let sheet_presence = smoothstep(threshold - width, threshold + width,
        input.discharge.z * (1.0 - 0.88 * thinness * taper));
    let pulses = mix(0.42, 1.0, smoothstep(0.26, 0.70, 0.7 * coarse + 0.3 * fine));
    let strand_presence = smoothstep(-0.12, 0.07, input.across_fraction.w)
        * mix(1.0, pulses, smoothstep(0.16, 0.77, input.across_fraction.w));
    let ruptures = 1.0 - 0.62 * smoothstep(0.55, 1.0, input.discharge.z);
    let breaks_apart = ruptures * smoothstep(0.45, 0.92, input.across_fraction.w);
    let feather = 1.0 - breaks_apart * smoothstep(0.50 + 0.19 * fine, 1.0, input.edge);
    let presence = select(sheet_presence * feather, strand_presence, is_filament);
    let spread = mix(0.63 * coarse + 0.37 * grouped, 0.5, smoothstep(0.9, 3.0, pixel));
    let contact = 0.85 + 0.055 * spread;
    let path_foot = 1.0 - smoothstep(contact, 0.98 + 0.02 * spread, input.across_fraction.w);
    let side = input.edge * input.edge;
    let rounding = select((0.018 + 0.13 * side) * (0.72 + 0.56 * fine), 0.0, is_filament);
    let contact_foam = smoothstep(0.10, 0.68, input.foam.x + 0.64 * input.foam.y);
    let ribbon_foot = smoothstep(rounding - 0.065 * contact_foam,
        rounding + select(0.17 + 0.21 * contact_foam, 0.06, is_filament),
        input.receiving.y);
    let strand_foot = smoothstep(0.03 + 0.16 * coarse, 0.36 + 0.19 * fine, input.receiving.y);
    let plunge_foot = mix(ribbon_foot, strand_foot, breakup);
    let meets_pool = input.receiving.z > 1.5;
    let foot = select(path_foot, plunge_foot, meets_pool);
    let contact_cover = foot
        * mix(1.0, smoothstep(0.18, 0.71, tear + 0.54 * fine),
        smoothstep(0.51, 0.90, input.across_fraction.w));
    let writes_depth = presence > 0.5 && foot > 0.35 && passing_cover > 0.28;
    if writes_depth != solid || presence * contact_cover * passing_cover < 0.015 {
        discard;
    }
    let seam = smoothstep(0.0, 0.05, input.world_time.w);
    let streak = sheet_noise(xy + vec2(0.11 * (coarse - fine), 0.0), span,
        vec2(4.0, 0.82), vec2(0.0)) - 0.5;
    let spray = sheet_noise(xy, span, vec2(13.1, 1.65), vec2(23.9)) - 0.5;
    let grain = mix(0.68 * streak + 0.32 * spray, 0.0, smoothstep(0.65, 2.8, pixel));
    let ripples = (0.92 * grain + 0.38 * (coarse - fine)) * cross(tangent, face)
        + 0.26 * spray * tangent;
    face = normalize(face + ripples * below_tongue);
    let nv = max(0.0, dot(face, look));
    let fresnel = 0.02 + 0.98 * pow(1.0 - nv, 5.0);
    let roughness = mix(0.12, mix(0.055, 0.39, white), below_tongue);
    let mirror = environment_reflection(reflect(-look, face), roughness, nv).radiance;
    let light = environment_sun_direction();
    let direct = sun_irradiance(point) * fall_cloud(point);
    let diffuse = mix(environment_diffuse(face), environment_diffuse(vec3(0.0, 1.0, 0.0)), 0.35);
    let foam = (diffuse + direct * (0.24 + 0.76 * max(dot(face, light), 0.0)))
        * (select(0.22, 0.34, is_filament) + 0.36 * grain);
    let translucence = mix(vec3(0.97, 1.0, 1.04), vec3(0.71, 0.94, 1.0), breakout * (0.36 - grain));
    let churn = foam * translucence * (0.60 + 0.72 * smoothstep(-0.44, 0.32, grain));
    let glint = pow(max(dot(reflect(-light, face), look), 0.0), 160.0) * 0.055 * direct;
    let glass = mirror * (0.18 + 0.82 * fresnel) + glint * below_tongue;
    let trickle_tongue = select(0.0, 0.44 * (1.0 - below_tongue), is_filament);
    let clear_light = mix(glass, diffuse * 0.22, trickle_tongue);
    let uv = input.clip.xy * view_aux.viewport_inv_viewport.zw;
    let aerial = environment_aerial(point, uv);
    let mixing = select(smoothstep(0.72, 0.99, input.across_fraction.w),
        1.0 - smoothstep(0.05, 0.58, input.receiving.y), meets_pool);
    let aerated = (environment_diffuse(vec3(0.0, 1.0, 0.0)) + direct * max(0.0, light.y))
        * vec3(0.84, 0.90, 0.89) / 3.14159265;
    let parcel_light = mix(0.16, 1.38, pulse);
    let broken_light = mix(churn, aerated * parcel_light, max(rolled, breakup * below_tongue));
    let round_light = 0.14 * chord;
    let bead_light = 0.20 * smoothstep(0.42, 0.78, droplets);
    let diffuse_streaks = 0.11 * coarse + 0.13 * fine + 0.12 * grain + bead_light;
    let packet_light = aerated * (0.26 + round_light + diffuse_streaks + 0.78 * packet);
    let sheet_colour = mix(mix(clear_light, broken_light, white), packet_light, mixed_air);
    let spray_light = mix(sheet_colour, aerated,
        mixing * (1.0 - thinness) * mix(0.32 + 0.58 * contact_foam + 0.10 * fine, 0.93, breakup));
    let colour = (spray_light * aerial.transmission + aerial.scatter) * environment_storage_scale();
    let streak_cover = mix(1.0, mix(0.48, 1.0, pulse),
        breakup * smoothstep(0.20, 0.72, input.across_fraction.w));
    let glass_alpha = min(mix(0.46, 0.88, below_tongue), fresnel + 0.07 + 1.7 * input.discharge.w + 0.35 * breakout);
    let trickle_film = select(1.0, mix(0.70, 1.0, smoothstep(-0.09, 0.19, fraction)), is_filament);
    let film_opacity = (clear * glass_alpha + white * (0.90 + 0.18 * grain)) * streak_cover;
    // The aerated body transmits through a similar thickness across its face.
    // Lighting follows the departing parcels independently of the cover.
    let glass_cover = film_opacity * presence * mix(0.62, 1.0, seam) * contact_cover * trickle_film;
    let breaking = smoothstep(0.47, 0.90, fraction)
        * (1.0 - smoothstep(0.28, 0.73, 0.40 * fine + 0.39 * coarse + 0.21 * droplets));
    let aerated_cover = 0.71 * foot * mix(0.16, 1.0, chord) * (1.0 - 0.54 * breaking);
    let opacity = mix(glass_cover, aerated_cover, mixed_air) * passing_cover;
    let path_distance = select(input.across_fraction.y, -input.across_fraction.y - 2.0, is_filament);
    let last_fraction = input.across_fraction.w - length(input.velocity_along.xyz) * sheet_clock.y
        / max(0.001, path_distance);
    let fold_delta = (fold_weight(input.across_fraction.w) - fold_weight(last_fraction))
        * select(select(1.0, 0.24, input.closed_strand != 0u), 0.18, is_filament);
    let displacement = input.velocity_along.xyz * sheet_clock.y
        + fold_delta * sheet_fold(lip, birth, across) + input.strand_motion + vec3(0.0, input.receiving.x, 0.0);
    let previous = view.previous_view_proj * vec4(point - displacement, 1.0);
    let current = view.unjittered_view_proj * vec4(point, 1.0);
    let motion = (previous.xy / previous.w - current.xy / current.w)
        * vec2(0.5, -0.5) * view_aux.viewport_inv_viewport.xy;
    return FallOutput(vec4(colour, opacity), select(motion, vec2(65504.0), sheet_clock.z > 0.5),
        white * presence * contact_cover * streak_cover * passing_cover);
}

@fragment fn sheet_thin(input: SheetVertex, @builtin(front_facing) front: bool) -> FallOutput {
    return sheet_shade(input, false, front);
}
@fragment fn sheet_solid(input: SheetVertex, @builtin(front_facing) front: bool) -> FallOutput {
    return sheet_shade(input, true, front);
}
