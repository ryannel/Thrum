struct CookedMeshPlacement {
    model: mat4x4<f32>,
    // History flag, meshlet count, cull enabled, largest axis scale (f32 bits).
    draw: vec4<u32>,
}
struct CookedMeshlet {
    vertex_offset: u32,
    triangle_offset: u32,
    vertex_count: u32,
    triangle_count: u32,
}
@group(4) @binding(0) var<uniform> mesh_placement: CookedMeshPlacement;
@group(4) @binding(1) var<storage, read> mesh_bounds: array<Bounds>;
@group(4) @binding(2) var<storage, read> mesh_positions: array<f32>;
@group(4) @binding(4) var<storage, read> mesh_table: array<CookedMeshlet>;
@group(4) @binding(5) var<storage, read> cooked_meshlet_vertices: array<u32>;
@group(4) @binding(6) var<storage, read> mesh_triangles: array<u32>;

struct CookedPayload { survivors: array<u32, 64> }
var<task_payload> cooked_payload: CookedPayload;

struct CookedPrimitive { @builtin(triangle_indices) indices: vec3<u32> }

fn mesh_position(vertex: u32) -> vec3<f32> {
    return vec3(mesh_positions[vertex], mesh_positions[vertex + 1u], mesh_positions[vertex + 2u]);
}

fn mesh_triangle_byte(index: u32) -> u32 {
    return (mesh_triangles[index / 4u] >> ((index % 4u) * 8u)) & 255u;
}

fn mesh_triangle(offset: u32) -> vec3<u32> {
    return vec3(mesh_triangle_byte(offset), mesh_triangle_byte(offset + 1u), mesh_triangle_byte(offset + 2u));
}
