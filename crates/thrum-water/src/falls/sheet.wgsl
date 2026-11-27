@group(3) @binding(0) var<uniform> sheet_clock: vec4<f32>;

struct SheetVertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_time: vec4<f32>,
    @location(1) velocity_along: vec4<f32>,
    @location(2) discharge: vec4<f32>,
    @location(3) across_fraction: vec4<f32>,
}

@vertex fn sheet_vertex(
    @location(0) world_time: vec4<f32>,
    @location(1) velocity_along: vec4<f32>,
    @location(2) discharge: vec4<f32>,
    @location(3) across_fraction: vec4<f32>,
) -> SheetVertex {
    return SheetVertex(view.view_proj * vec4(world_time.xyz, 1.0), world_time, velocity_along, discharge, across_fraction);
}

struct FallOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
}

fn sheet_age(age: f32) -> f32 { return age - floor(age / FALL_LOOP) * FALL_LOOP; }

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
    let width = max(0.025, min(0.27, 0.10 * pixel));
    let tangent = normalize(input.velocity_along.xyz + vec3(1e-8, -1e-8, 0.0));
    let normal = normalize(cross(input.across_fraction.xyz, tangent));
    var face = select(-normal, normal, dot(normal, look) >= 0.0);
    let breakout = input.discharge.y;
    let broken = smoothstep(rank - 0.24 - width, rank + 0.09 + width, breakout);
    let white = mix(pow(breakout, 1.18), broken, 0.74);
    let clear = 1.0 - white;
    // Preserve the long strands of a small discharge when the trail widens
    // in time. The lower-frequency mask follows each parcel from the lip.
    let rivulet = sheet_noise(vec2(lip, birth), FALL_LOOP, vec2(3.2, 0.28), vec2(18.0, 0.0));
    let tear = pow(0.84 * rivulet + 0.16 * coarse, 3.2);
    let threshold = mix(tear, 0.32, smoothstep(1.5, 4.0, pixel));
    let presence = smoothstep(threshold - width, threshold + width, input.discharge.z);
    let spread = mix(0.63 * coarse + 0.37 * grouped, 0.5, smoothstep(0.9, 3.0, pixel));
    let contact = 0.65 + 0.22 * spread;
    let foot = 1.0 - smoothstep(contact, 1.0, input.across_fraction.w);
    if (presence > 0.5 && foot > 0.35) != solid || presence * foot < 0.015 {
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
    let aerated = diffuse * vec3(0.17, 0.47, 0.48) + direct * vec3(0.14, 0.32, 0.33);
    let sheet_colour = mix(glass, churn, white);
    let spray_light = mix(sheet_colour, aerated, mixing * white * (0.27 + 0.34 * fine));
    let colour = (spray_light * aerial.transmission + aerial.scatter) * environment_storage_scale();
    let opacity = (clear * min(0.88, fresnel + 0.07 + 1.7 * input.discharge.w + 0.35 * breakout) + white * (0.90 + 0.18 * grain))
        * presence * mix(0.62, 1.0, seam) * foot;
    let displacement = input.velocity_along.xyz * sheet_clock.y;
    let previous = view.previous_view_proj * vec4(point - displacement, 1.0);
    let current = view.unjittered_view_proj * vec4(point, 1.0);
    let motion = (previous.xy / previous.w - current.xy / current.w)
        * vec2(0.5, -0.5) * view_aux.viewport_inv_viewport.xy;
    return FallOutput(vec4(colour, opacity), select(motion, vec2(65504.0), sheet_clock.z > 0.5), white * presence * foot);
}

@fragment fn sheet_thin(input: SheetVertex) -> FallOutput { return sheet_shade(input, false); }
@fragment fn sheet_solid(input: SheetVertex) -> FallOutput { return sheet_shade(input, true); }
