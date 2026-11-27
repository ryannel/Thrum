@group(0) @binding(3) var<uniform> environment_light: EnvironmentLight;
@group(0) @binding(7) var overlay_faces: texture_2d_array<f32>;
@group(0) @binding(8) var<storage, read_write> overlay_chart: ChartLighting;
var<workgroup> cloud_flags: array<u32, 64>;
var<workgroup> active_overlay: u32;

fn classify_overlay(index: u32) -> bool {
    cloud_flags[index] = u32(overlay_face_has_cloud(index));
    workgroupBarrier();
    if index == 0u {
        active_overlay = 0u;
        for (var i = 0u; i < 64u; i++) { active_overlay |= cloud_flags[i]; }
        overlay_chart.cloudy = active_overlay;
        if overlay_chart.mode == 2u {
            let resident = u32(environment_light.sky_diffuse[2].w != 0.0);
            overlay_chart.reflection_tags[0] = vec4(resident);
            overlay_chart.reflection_tags[1] = vec4(resident);
        }
    }
    return workgroupUniformLoad(&active_overlay) != 0u;
}

@compute @workgroup_size(64)
fn classify_main(@builtin(local_invocation_index) index: u32) {
    classify_overlay(index);
}
