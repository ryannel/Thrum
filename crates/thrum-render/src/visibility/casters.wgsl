
// Sun caster culling (G3c M3c, experimental): one member-indexed compaction
// per Sun pass, against that pass's own light matrix, never the camera's.
// A member is listed unless its conservative world box (the same interpolated
// translation as the caster vertex stage) lies wholly outside the volume
// that pass rasterizes: the xy clip planes, and in depth what `shadow.wgsl`
// keeps. The caster stage clamps z below 0 to 0, which the Greater test
// against the 0 clear never passes, so the z = 0 plane always rejects. It
// clamps z above w to w (still drawn) unless the pass clips upstream
// geometry, when z = w is a real clip plane too.
// `CasterView` is `ShadowView` in `shadow.wgsl`: the bytes the pass draws with.
struct CasterView {
    light_view_proj: mat4x4<f32>,
    clip_upstream: vec4<u32>,
}
@group(0) @binding(14) var<uniform> caster_view: CasterView;

fn caster_row(row: u32) -> vec4<f32> {
    let m = caster_view.light_view_proj;
    return vec4(m[0][row], m[1][row], m[2][row], m[3][row]);
}

// Wholly behind `plane`, past a margin relative to the magnitudes both the
// box and the vertex stage sum: rounding cannot move a drawn vertex across.
fn caster_outside(plane: vec4<f32>, center: vec3<f32>, extent: vec3<f32>) -> bool {
    let distance = dot(plane.xyz, center) + plane.w;
    let radius = dot(abs(plane.xyz), extent);
    let magnitude = dot(abs(plane.xyz), abs(center) + extent) + abs(plane.w);
    return distance + radius + 1.0e-5 * magnitude < 0.0;
}

fn casts(slot: u32, mesh: MeshRecord) -> bool {
    // The caster stage collapses inactive rows to a point outside the clip.
    if instances[slot].ids.w == 0u {
        return false;
    }
    let bounds = world_bounds(slot, mesh);
    let center = bounds.center;
    let extent = bounds.extent;
    let r0 = caster_row(0u);
    let r1 = caster_row(1u);
    let r2 = caster_row(2u);
    let r3 = caster_row(3u);
    let upstream = caster_view.clip_upstream.x != 0u && caster_outside(r3 - r2, center, extent);
    return !(caster_outside(r3 + r0, center, extent) || caster_outside(r3 - r0, center, extent)
        || caster_outside(r3 + r1, center, extent) || caster_outside(r3 - r1, center, extent)
        || caster_outside(r2, center, extent) || upstream);
}

@compute @workgroup_size(256)
fn count_casters(@builtin(global_invocation_id) global: vec3<u32>,
                 @builtin(local_invocation_id) local: vec3<u32>,
                 @builtin(workgroup_id) group: vec3<u32>) {
    if local.x == 0u {
        group_mesh = buckets.segments[group_bucket(group.x)].mesh;
    }
    let mesh = meshes[workgroupUniformLoad(&group_mesh)];
    let slot = members[global.x];
    var draw = false;
    if slot != MEMBER_PADDING {
        draw = casts(slot, mesh);
    }
    let word = count_word(draw, local.x, group.x);
    if local.x < WORDS {
        group_offsets[buckets.mask_base + group.x * WORDS + local.x] = word;
    }
}
