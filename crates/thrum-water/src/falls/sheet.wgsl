@group(3) @binding(0) var<uniform> sheet_clock: vec4<f32>;

struct SheetVertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_time: vec4<f32>,
    @location(1) velocity_along: vec4<f32>,
    @location(2) discharge: vec4<f32>,
    @location(3) across_fraction: vec4<f32>,
    @location(4) edge: f32,
}

fn sheet_age(age: f32) -> f32 { return age - floor(age / FALL_LOOP) * FALL_LOOP; }

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
    let strength = smoothstep(0.007, 0.065, discharge.w);
    let axis = across_fraction.xz;
    let across = vec3(axis.x, 0.0, axis.y);
    let face = vec3(-axis.y, 0.0, axis.x);
    let flow = velocity_along.xyz * vec3(1.0, 0.0, 1.0);
    let outward = select(-face, face, dot(face, flow) >= 0.0);
    let drift = outward * mix(0.13, 0.59, kick) + across * (random - 0.5) * 0.54;
    let loft = mix(0.034, 0.39, kick * kick) * mix(0.30, 1.0, strength);
    let centre = world_time.xyz + drift * phase + vec3(0.0, 4.0 * loft * phase * (1.0 - phase), 0.0);
    let eye = normalize(view.camera_xyz_time.xyz - centre);
    let climb = drift + vec3(0.0, 4.0 * loft * (1.0 - 2.0 * phase), 0.0);
    let right = normalize(cross(eye, climb) + vec3(1e-6, 0.0, 0.0));
    let up = cross(right, eye);
    let index = u32(across_fraction.w);
    let corner = vec2(f32(index & 1u), f32(index >> 1u)) * 2.0 - 1.0;
    let radius = mix(0.018, 0.089, random * random) * mix(0.45, 1.0, strength);
    let point = centre + radius * (right * corner.x + up * corner.y * (1.0 + 1.3 * kick));
    let world = vec4(point, phase);
    return SheetVertex(view.view_proj * vec4(point, 1.0), world, velocity_along,
        vec4(random, kick, strength, discharge.w), vec4(corner.x, -1.0, corner.y, phase), 0.0);
}

@vertex fn sheet_vertex(
    @location(0) world_time: vec4<f32>,
    @location(1) velocity_along: vec4<f32>,
    @location(2) discharge: vec4<f32>,
    @location(3) across_fraction: vec4<f32>,
    @location(4) edge: f32,
) -> SheetVertex {
    let is_splash = across_fraction.y < 0.0 && across_fraction.y > -1.5;
    if is_splash { return splash_vertex(world_time, velocity_along, discharge, across_fraction); }
    let birth = sheet_age(sheet_clock.x - world_time.w);
    let is_filament = across_fraction.y <= -2.0;
    let weight = fold_weight(across_fraction.w) * select(1.0, 0.18, is_filament);
    let fold = sheet_fold(velocity_along.w, birth, across_fraction.xz) * weight;
    let world = vec4(world_time.xyz + fold, world_time.w);
    return SheetVertex(view.view_proj * vec4(world.xyz, 1.0), world, velocity_along, discharge, across_fraction, edge);
}

struct FallOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
}

fn splash_shade(input: SheetVertex) -> FallOutput {
    let phase = input.across_fraction.w;
    let corner = input.across_fraction.xz;
    let support = 1.0 - smoothstep(0.1, 1.0, dot(corner, corner));
    let alive = smoothstep(0.0, 0.15, phase) * (1.0 - smoothstep(0.69, 1.0, phase));
    let alpha = input.discharge.z * support * alive * mix(0.32, 0.92, input.discharge.x);
    if alpha < 0.012 { discard; }
    let up = vec3(0.0, 1.0, 0.0);
    let point = input.world_time.xyz;
    let direct = sun_irradiance(point) * fall_cloud(point) * max(0.0, environment_sun_direction().y);
    let foam = (environment_diffuse(up) + direct) * vec3(0.82, 0.89, 0.88) / 3.14159265;
    let air = environment_aerial(point, input.clip.xy * view_aux.viewport_inv_viewport.zw);
    let colour = (foam * air.transmission + air.scatter) * environment_storage_scale();
    return FallOutput(vec4(colour, alpha), vec2(0.0), alpha);
}

fn sheet_shade(input: SheetVertex, solid: bool) -> FallOutput {
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
    if input.across_fraction.y < 0.0 && input.across_fraction.y > -1.5 {
        if solid { discard; }
        return splash_shade(input);
    }
    let width = max(0.025, min(0.27, 0.10 * pixel));
    let tangent = normalize(input.velocity_along.xyz + vec3(1e-8, -1e-8, 0.0));
    let across = input.across_fraction.xz;
    let is_filament = input.across_fraction.y <= -2.0;
    let normal = normalize(cross(vec3(across.x, 0.0, across.y), tangent));
    let geometric = select(-geometry_normal, geometry_normal, dot(geometry_normal, normal) > 0.0);
    let contour = normalize(mix(normal, geometric,
        select(0.38 * smoothstep(0.34, 0.76, input.across_fraction.w), 1.0, is_filament)));
    var face = select(-contour, contour, dot(contour, look) >= 0.0);
    let breakout = input.discharge.y;
    let broken = smoothstep(rank - 0.24 - width, rank + 0.09 + width, breakout);
    let filament_white = smoothstep(0.06, 0.48, input.across_fraction.w)
        * smoothstep(0.08, 0.58, 0.72 * coarse + 0.28 * fine);
    let streak_white = smoothstep(0.42, 0.89, input.across_fraction.w)
        * smoothstep(0.23, 0.78, 0.65 * coarse + 0.35 * fine);
    let white = select(max(mix(pow(breakout, 1.18), broken, 0.74), 0.63 * streak_white),
        filament_white, is_filament);
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
        * (1.0 - smoothstep(0.73, 1.0, input.across_fraction.w))
        * mix(1.0, pulses, smoothstep(0.16, 0.77, input.across_fraction.w));
    let breaks_apart = smoothstep(0.45, 0.92, input.across_fraction.w);
    let feather = 1.0 - breaks_apart * smoothstep(0.50 + 0.19 * fine, 1.0, input.edge);
    let presence = select(sheet_presence * feather, strand_presence, is_filament);
    let spread = mix(0.63 * coarse + 0.37 * grouped, 0.5, smoothstep(0.9, 3.0, pixel));
    let contact = 0.56 + 0.24 * spread;
    let foot = 1.0 - smoothstep(contact, 0.91 + 0.09 * spread, input.across_fraction.w);
    let contact_cover = mix(0.21 * (1.0 - thinness), 1.0, foot)
        * mix(1.0, smoothstep(0.18, 0.71, tear + 0.54 * fine),
        smoothstep(0.51, 0.90, input.across_fraction.w));
    if (presence > 0.5 && foot > 0.35) != solid || presence * contact_cover < 0.015 {
        discard;
    }
    let seam = smoothstep(0.0, 0.05, input.world_time.w);
    let streak = sheet_noise(xy + vec2(0.11 * (coarse - fine), 0.0), span,
        vec2(4.0, 0.82), vec2(0.0)) - 0.5;
    let spray = sheet_noise(xy, span, vec2(13.1, 1.65), vec2(23.9)) - 0.5;
    let grain = mix(0.68 * streak + 0.32 * spray, 0.0, smoothstep(0.65, 2.8, pixel));
    face = normalize(face + (0.92 * grain + 0.38 * (coarse - fine)) * cross(tangent, face)
        + 0.26 * spray * tangent);
    let nv = max(0.0, dot(face, look));
    let fresnel = 0.02 + 0.98 * pow(1.0 - nv, 5.0);
    let mirror = environment_reflection(reflect(-look, face), mix(0.055, 0.39, white), nv).radiance;
    let light = environment_sun_direction();
    let direct = sun_irradiance(point) * fall_cloud(point);
    let diffuse = mix(environment_diffuse(face), environment_diffuse(vec3(0.0, 1.0, 0.0)), 0.35);
    let foam = (diffuse + direct * (0.24 + 0.76 * max(dot(face, light), 0.0))) * (0.22 + 0.36 * grain);
    let translucence = mix(vec3(0.97, 1.0, 1.04), vec3(0.71, 0.94, 1.0), breakout * (0.36 - grain));
    let churn = foam * translucence * (0.60 + 0.72 * smoothstep(-0.44, 0.32, grain));
    let glint = pow(max(dot(reflect(-light, face), look), 0.0), 160.0) * 0.055 * direct;
    let glass = mirror * (0.18 + 0.82 * fresnel) + glint;
    let uv = input.clip.xy * view_aux.viewport_inv_viewport.zw;
    let aerial = environment_aerial(point, uv);
    let mixing = smoothstep(0.63, 1.0, input.across_fraction.w);
    let aerated = (diffuse + 0.46 * direct) * vec3(0.29, 0.36, 0.36);
    let sheet_colour = mix(glass, churn, white);
    let spray_light = mix(sheet_colour, aerated, mixing * white * (0.55 + 0.38 * fine));
    let colour = (spray_light * aerial.transmission + aerial.scatter) * environment_storage_scale();
    let opacity = (clear * min(0.88, fresnel + 0.07 + 1.7 * input.discharge.w + 0.35 * breakout) + white * (0.90 + 0.18 * grain))
        * presence * mix(0.62, 1.0, seam) * contact_cover;
    let path_distance = select(input.across_fraction.y, -input.across_fraction.y - 2.0, is_filament);
    let last_fraction = input.across_fraction.w - length(input.velocity_along.xyz) * sheet_clock.y
        / max(0.001, path_distance);
    let fold_delta = (fold_weight(input.across_fraction.w) - fold_weight(last_fraction))
        * select(1.0, 0.18, is_filament);
    let displacement = input.velocity_along.xyz * sheet_clock.y + fold_delta * sheet_fold(lip, birth, across);
    let previous = view.previous_view_proj * vec4(point - displacement, 1.0);
    let current = view.unjittered_view_proj * vec4(point, 1.0);
    let motion = (previous.xy / previous.w - current.xy / current.w)
        * vec2(0.5, -0.5) * view_aux.viewport_inv_viewport.xy;
    return FallOutput(vec4(colour, opacity), select(motion, vec2(65504.0), sheet_clock.z > 0.5), white * presence * contact_cover);
}

@fragment fn sheet_thin(input: SheetVertex) -> FallOutput { return sheet_shade(input, false); }
@fragment fn sheet_solid(input: SheetVertex) -> FallOutput { return sheet_shade(input, true); }
