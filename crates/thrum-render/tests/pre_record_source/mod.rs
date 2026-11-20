//! Project the record composition and generated constants back to the shader source
//! used by the retained SCENE-VIEWS fingerprints. Each projected fragment is checked.

const START: &str = "\n// thrum-wgsl-source: ";
const END: &str = "\n// thrum-wgsl-end\n";

fn replace_once(source: &mut String, current: &str, previous: &str) {
    assert_eq!(
        source.matches(current).count(),
        1,
        "M2 WGSL fragment {current:?}"
    );
    *source = source.replacen(current, previous, 1);
}

fn take_record(source: &mut String, file: &str) -> String {
    let marker = format!("{START}records/{file}.wgsl\n");
    let start = source.find(&marker).expect("composed record");
    let body = start + marker.len();
    let end = body + source[body..].find(END).expect("end of record");
    let record = source[body..end].to_owned();
    source.replace_range(start..end + END.len(), "");
    record
}

pub(super) fn before_record_composition(text: &str) -> String {
    let mut source = text.to_owned();
    let view = take_record(&mut source, "view");
    let view_aux = take_record(&mut source, "view_aux");
    let mut instance = take_record(&mut source, "instance");
    let serials = take_record(&mut source, "serials");
    let mut mesh_vertex = take_record(&mut source, "mesh_vertex");
    let history = take_record(&mut source, "history");
    let shadow_cascades = take_record(&mut source, "shadow_cascades");
    let cluster_head = take_record(&mut source, "cluster_head");
    replace_once(
        &mut instance,
        "    normal: array<vec4<f32>, 3>,\n",
        "    normal0: vec4<f32>,\n    normal1: vec4<f32>,\n    normal2: vec4<f32>,\n",
    );
    replace_once(
        &mut mesh_vertex,
        "{\n    a: vec4<f32>,\n    b: vec4<f32>,\n    c: vec4<f32>,\n}",
        "{ a: vec4<f32>, b: vec4<f32>, c: vec4<f32> }",
    );
    replace_once(
        &mut source,
        "@group(0) @binding(0) var<uniform> view: ViewRecord;",
        &format!("{view}@group(0) @binding(0) var<uniform> view: ViewRecord;"),
    );
    replace_once(
        &mut source,
        "@group(0) @binding(0) var<uniform> view: ViewRecord;",
        &format!("{view_aux}@group(0) @binding(0) var<uniform> view: ViewRecord;"),
    );
    replace_once(
        &mut source,
        "@group(2) @binding(0) var<storage, read> materials: array<MaterialRecord>;",
        &format!(
            "{instance}@group(2) @binding(0) var<storage, read> materials: array<MaterialRecord>;"
        ),
    );
    replace_once(
        &mut source,
        "@group(3) @binding(1) var<storage, read> history: array<HistoryRecord>;",
        &format!(
            "{history}@group(3) @binding(1) var<storage, read> history: array<HistoryRecord>;"
        ),
    );
    replace_once(
        &mut source,
        "@group(3) @binding(3) var<storage, read> mesh_vertices: array<MeshVertex>;",
        &format!(
            "{mesh_vertex}@group(3) @binding(3) var<storage, read> mesh_vertices: array<MeshVertex>;"
        ),
    );
    replace_once(
        &mut source,
        "@group(3) @binding(1) var<storage, read> history: array<HistoryRecord>;",
        &format!(
            "{serials}@group(3) @binding(1) var<storage, read> history: array<HistoryRecord>;"
        ),
    );
    replace_once(
        &mut source,
        "@group(0) @binding(2) var<uniform> shadows: ShadowCascades;",
        &format!("{shadow_cascades}@group(0) @binding(2) var<uniform> shadows: ShadowCascades;"),
    );
    replace_once(
        &mut source,
        "@group(1) @binding(1) var<storage, read> local_lights: array<LocalSource>;",
        &format!(
            "{cluster_head}@group(1) @binding(1) var<storage, read> local_lights: array<LocalSource>;"
        ),
    );
    restore_aerial(&mut source);
    replace_once(
        &mut source,
        "const REC709_LUMA = vec3<f32>(0.2126,0.7152,0.0722);\nconst MAX_FINITE_HALF: f32 = 65504.0;\n",
        "",
    );
    for (current, previous) in [
        ("vec3(MAX_FINITE_HALF)", "vec3(65504.0)"),
        ("return vec2(MAX_FINITE_HALF);", "return vec2(65504.0);"),
        (
            "vec2(-MAX_FINITE_HALF), vec2(MAX_FINITE_HALF)",
            "vec2(-65504.0), vec2(65504.0)",
        ),
    ] {
        replace_once(&mut source, current, previous);
    }
    if source.contains("fn air_sun_visibility(") {
        replace_once(
            &mut source,
            "dot(delta.rgb, REC709_LUMA)",
            "dot(delta.rgb, vec3(0.2126, 0.7152, 0.0722))",
        );
    }
    replace_once(
        &mut source,
        "const SHADOW_HEIGHT_LAYER: u32 = 2u;\nconst SUN_TAN_RADIUS: f32 = 0.004654244;\n",
        "",
    );
    replace_once(
        &mut source,
        "// A continuous surface inside the filter footprint",
        "const SUN_TAN_RADIUS: f32 = 0.004654244;\n// A continuous surface inside the filter footprint",
    );
    replace_once(
        &mut source,
        "textureLoad(shadow_maps,texel_index,i32(SHADOW_HEIGHT_LAYER),0)",
        "textureLoad(shadow_maps,texel_index,2,0)",
    );
    for (current, previous) in [
        (
            r"    output.normal = mat3x3(instance.normal[0].xyz,instance.normal[1].xyz,instance.normal[2].xyz) * n;
",
            r"    output.normal = mat3x3(instance.normal0.xyz,instance.normal1.xyz,instance.normal2.xyz) * n;
",
        ),
        (
            r"        u32(clamp(floor((world_position.x+1.0)* (0.5 * f32(CLUSTER_X))),0.0,f32(CLUSTER_X - 1u))),
        u32(clamp(floor((world_position.y+1.0)* (0.5 * f32(CLUSTER_Y))),0.0,f32(CLUSTER_Y - 1u))),
        u32(clamp(floor(world_position.z*f32(CLUSTER_Z)),0.0,f32(CLUSTER_Z - 1u))));
",
            r"        u32(clamp(floor((world_position.x+1.0)*16.0),0.0,31.0)),
        u32(clamp(floor((world_position.y+1.0)*9.0),0.0,17.0)),
        u32(clamp(floor(world_position.z*16.0),0.0,15.0)));
",
        ),
        (
            r"            u32(clamp(floor(screen_position.x*view_aux.viewport_inv_viewport.z*f32(CLUSTER_X)),0.0,f32(CLUSTER_X - 1u))),
            u32(clamp(floor((1.0-screen_position.y*view_aux.viewport_inv_viewport.w)*f32(CLUSTER_Y)),0.0,f32(CLUSTER_Y - 1u))),
            u32(clamp(floor(log(depth/near)/log(far/near)*f32(CLUSTER_Z)),0.0,f32(CLUSTER_Z - 1u))));
",
            r"            u32(clamp(floor(screen_position.x*view_aux.viewport_inv_viewport.z*32.0),0.0,31.0)),
            u32(clamp(floor((1.0-screen_position.y*view_aux.viewport_inv_viewport.w)*18.0),0.0,17.0)),
            u32(clamp(floor(log(depth/near)/log(far/near)*16.0),0.0,15.0)));
",
        ),
        (
            r"    let head = cluster_heads[(cluster.z*CLUSTER_Y+cluster.y)*CLUSTER_X+cluster.x];
",
            r"    let head = cluster_heads[(cluster.z*18u+cluster.y)*32u+cluster.x];
",
        ),
    ] {
        replace_once(&mut source, current, previous);
    }
    for (current, previous) in [
        (
            "// Transport remains per unit Sun; consumers decode before lighting/exposure.\n",
            "// Transport remains per unit Sun; consumers decode before lighting/exposure.\nconst SKY_LUT_GAIN: f32 = 1024.0;\n",
        ),
        (
            r"fn decode_sky_light(encoded: vec4<f32>, solar: vec3<f32>) -> vec3<f32> {
",
            r"const NIGHT_LUT_GAIN: f32 = 65536.0;
fn decode_sky_light(encoded: vec4<f32>, solar: vec3<f32>) -> vec3<f32> {
",
        ),
    ] {
        replace_once(&mut source, current, previous);
    }
    replace_once(
        &mut source,
        "const CLUSTER_X: u32 = 32u;\nconst CLUSTER_Y: u32 = 18u;\nconst CLUSTER_Z: u32 = 16u;\nconst CLUSTER_WORKGROUP: u32 = 64u;\n",
        "",
    );
    replace_once(
        &mut source,
        "const AERIAL_INTERVALS: u32 = 32u;\nconst SKY_LUT_GAIN: f32 = 1024.0;\nconst NIGHT_LUT_GAIN: f32 = 65536.0;\n",
        "",
    );
    while let Some(start) = source.find(START) {
        let line_end = source[start + START.len()..]
            .find('\n')
            .expect("end of source marker");
        source.replace_range(start..=start + START.len() + line_end, "");
    }
    source.replace(END, "")
}

fn restore_aerial(source: &mut String) {
    if source.contains("@group(1) @binding(7) var<uniform> aerial_frame: Atmosphere;") {
        let mut observer = take_record(source, "observer");
        replace_once(&mut observer, "struct Atmosphere", "struct AerialFrame");
        replace_once(
            &mut observer,
            "    sky_mapping: vec4<f32>, // Horizon mu/elevation, sky/ground angular spans.",
            "    sky_mapping: vec4<f32>, // Mirrors Atmosphere; screen extent belongs to ViewAux.",
        );
        let first = "@group(1) @binding(4) var aerial_scatter: texture_2d_array<f32>;";
        replace_once(source, first, &format!("{observer}{first}"));
        for (current, previous) in [
            (
                r"@group(1) @binding(7) var<uniform> aerial_frame: Atmosphere;
",
                r"@group(1) @binding(7) var<uniform> aerial_frame: AerialFrame;
",
            ),
            (
                r"    let coordinate = sqrt(distance / 32000.0) * f32(AERIAL_INTERVALS);
    let low = min(AERIAL_INTERVALS - 1u, u32(floor(coordinate)));
",
                r"    let coordinate = sqrt(distance / 32000.0) * 32.0;
    let low = min(31u, u32(floor(coordinate)));
",
            ),
            (
                r"    let first_distance = 32000.0 * pow(f32(low) / f32(AERIAL_INTERVALS), 2.0);
    let second_distance = 32000.0 * pow(f32(high) / f32(AERIAL_INTERVALS), 2.0);
",
                r"    let first_distance = 32000.0 * pow(f32(low) / 32.0, 2.0);
    let second_distance = 32000.0 * pow(f32(high) / 32.0, 2.0);
",
            ),
        ] {
            replace_once(source, current, previous);
        }
        replace_once(
            source,
            "const TRANS_PLANES: u32 = 3u;\nconst SPECTRAL_PLANES: u32 = 2u;\nconst R: f32 = 6371000.0;\nconst H: f32 = 100000.0;\n",
            "",
        );
        for (current, previous) in [
            (
                r"fn altitude(p: vec3<f32>) -> f32 {
",
                r"const R: f32 = 6371000.0;
const H: f32 = 100000.0;
fn altitude(p: vec3<f32>) -> f32 {
",
            ),
            (
                r"fn plane_size(size: vec2<u32>, planes: u32) -> vec2<u32> {
",
                r"const TRANS_PLANES: u32 = 3u;
const SPECTRAL_PLANES: u32 = 2u;
fn plane_size(size: vec2<u32>, planes: u32) -> vec2<u32> {
",
            ),
        ] {
            replace_once(source, current, previous);
        }
    }
}
