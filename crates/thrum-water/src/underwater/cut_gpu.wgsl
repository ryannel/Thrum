struct LensFace {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
    rest_z: vec3<f32>,
    ownership: u32,
    river: vec4<f32>,
}
struct LensWork {
    count: vec4<u32>,
    bounds: vec4<f32>,
    cells: array<vec4<u32>>,
}
struct LensWrite {
    count: atomic<u32>,
    padding0: u32,
    padding: vec2<u32>,
    faces: array<LensFace>,
}
struct LensRead {
    count: u32,
    padding0: u32,
    padding: vec2<u32>,
    faces: array<LensFace>,
}
@group(2) @binding(25) var<storage, read_write> lens_write: LensWrite;
@group(3) @binding(22) var<storage, read> lens_faces: LensRead;
@group(3) @binding(23) var<storage, read> lens_mesh: array<vec4<f32>>;
@group(3) @binding(24) var<storage, read> lens_work: LensWork;

fn lens_face(a: VertexOut, b: VertexOut, c: VertexOut) -> LensFace {
    let transformed = select(vec3(0u), vec3(1u), vec3(a.surf_weight, b.surf_weight, c.surf_weight) > vec3(0.0));
    let ownership = a.body | transformed.x << 8u | transformed.y << 9u | transformed.z << 10u;
    return LensFace(vec4(a.world, a.rest.x), vec4(b.world, b.rest.x),
        vec4(c.world, c.rest.x), vec3(a.rest.y, b.rest.y, c.rest.y), ownership,
        lens_river_data(a, b, c));
}

fn lens_vertices(cell: u32) -> array<VertexOut, 4> {
    let nodes = lens_work.cells[cell];
    return array(displaced_water_vertex(lens_mesh[nodes.x], nodes.x),
        displaced_water_vertex(lens_mesh[nodes.y], nodes.y),
        displaced_water_vertex(lens_mesh[nodes.z], nodes.z),
        displaced_water_vertex(lens_mesh[nodes.w], nodes.w));
}

fn admit_lens_face(a: VertexOut, b: VertexOut, c: VertexOut) {
    let lo = min(a.world.xz, min(b.world.xz, c.world.xz));
    let hi = max(a.world.xz, max(b.world.xz, c.world.xz));
    let pad = max(max(abs(lo), abs(hi)), vec2(1.0)) * 0.000001;
    if any(hi + pad < lens_work.bounds.xy) || any(lo - pad > lens_work.bounds.zw) { return; }
    let index = atomicAdd(&lens_write.count, 1u);
    if index < arrayLength(&lens_write.faces) {
        lens_write.faces[index] = lens_face(a, b, c);
    }
}

@compute @workgroup_size(64)
fn displace_lens_faces(@builtin(global_invocation_id) id: vec3<u32>) {
    let cell = id.x + lens_work.count.y * id.y;
    if cell >= lens_work.count.x { return; }
    let nodes = lens_vertices(cell);
    admit_lens_face(nodes[0], nodes[2], nodes[1]);
    admit_lens_face(nodes[1], nodes[2], nodes[3]);
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
    let height = face.a.y + b * (face.b.y-face.a.y) + c * (face.c.y-face.a.y);
    // In a fold, faces landing in one column can come from different rows.
    let rest_a = vec2(face.a.w, face.rest_z.x);
    let rest_b = vec2(face.b.w, face.rest_z.y);
    let rest_c = vec2(face.c.w, face.rest_z.z);
    let rest = rest_a + b * (rest_b-rest_a) + c * (rest_c-rest_a);
    return lens_sheet_height(face, vec3(1.0 - b - c, b, c), point, rest, height, current);
}

fn highest_lens_face(point: vec2<f32>) -> f32 {
    var highest = -1e30;
    if lens_faces.count <= arrayLength(&lens_faces.faces) {
        for (var i = 0u; i < lens_faces.count; i++) {
            highest = lens_face_height(lens_faces.faces[i], point, highest);
        }
    } else {
        // A densely folded footprint can exceed the compact face buffer.
        // Resolve that pixel from the same submitted vertices and topology.
        for (var cell = 0u; cell < lens_work.count.x; cell++) {
            let nodes = lens_vertices(cell);
            highest = lens_face_height(lens_face(nodes[0], nodes[2], nodes[1]), point, highest);
            highest = lens_face_height(lens_face(nodes[1], nodes[2], nodes[3]), point, highest);
        }
    }
    return highest;
}
