struct LensFace {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
}
struct LensWork {
    count: vec4<u32>,
    bounds: vec4<f32>,
    cells: array<vec4<u32>>,
}
struct LensWrite {
    count: atomic<u32>,
    enabled: u32,
    padding: vec2<u32>,
    faces: array<LensFace>,
}
struct LensRead {
    count: u32,
    enabled: u32,
    padding: vec2<u32>,
    faces: array<LensFace>,
}
@group(2) @binding(23) var<storage, read> lens_mesh: array<vec4<f32>>;
@group(2) @binding(24) var<storage, read> lens_work: LensWork;
@group(2) @binding(25) var<storage, read_write> lens_write: LensWrite;
@group(3) @binding(22) var<storage, read> lens_faces: LensRead;

fn admit_lens_face(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>) {
    let lo = min(a.xz, min(b.xz, c.xz));
    let hi = max(a.xz, max(b.xz, c.xz));
    let pad = max(max(abs(lo), abs(hi)), vec2(1.0)) * 0.000001;
    if any(hi + pad < lens_work.bounds.xy) || any(lo - pad > lens_work.bounds.zw) { return; }
    let index = atomicAdd(&lens_write.count, 1u);
    lens_write.faces[index] = LensFace(vec4(a, 0.0), vec4(b, 0.0), vec4(c, 0.0));
}

@compute @workgroup_size(64)
fn displace_lens_faces(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= lens_work.count.x { return; }
    let nodes = lens_work.cells[id.x];
    let a = displaced_water_vertex(lens_mesh[nodes.x], nodes.x).world;
    let b = displaced_water_vertex(lens_mesh[nodes.y], nodes.y).world;
    let c = displaced_water_vertex(lens_mesh[nodes.z], nodes.z).world;
    let d = displaced_water_vertex(lens_mesh[nodes.w], nodes.w).world;
    admit_lens_face(a, c, b);
    admit_lens_face(b, c, d);
}

fn lens_cross(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn lens_face_height(face: LensFace, point: vec2<f32>, current: f32) -> f32 {
    let ab = face.b.xz - face.a.xz;
    let ac = face.c.xz - face.a.xz;
    let at = point - face.a.xz;
    let determinant = lens_cross(ab, ac);
    if determinant == 0.0 { return current; }
    let b = lens_cross(at, ac) / determinant;
    let c = lens_cross(ab, at) / determinant;
    if min(min(b, c), 1.0-b-c) < -0.000001 { return current; }
    return max(current, face.a.y + b * (face.b.y-face.a.y) + c * (face.c.y-face.a.y));
}
