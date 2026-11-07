//! glTF to meshlet mesh product.
//!
//! The default scene is flattened into one static mesh in glTF's metres, Y-up, right-handed frame
//! (which is Thrum's C01 convention, so no conversion is needed). Each triangle primitive becomes a
//! submesh. Vertices are welded, then each submesh is vertex-cache and overdraw optimized,
//! vertex fetch is reordered, and meshoptimizer splits it into meshlets with culling bounds. The
//! portable index buffer is emitted in meshlet order.

use std::path::Path;

use thrum_asset::layout::{
    MESH_MAGIC, MeshMeta, Meshlet, MeshletBounds, Submesh, VertexAttributes, mesh_tag,
};

use crate::CookError;
use crate::write::ContainerWriter;

/// Meshlet vertex limit; 64 suits mesh-shader hardware and compute-expanded portable culling.
pub(crate) const MESHLET_VERTICES: usize = 64;
/// Meshlet triangle limit; a multiple of four as meshoptimizer requires.
pub(crate) const MESHLET_TRIANGLES: usize = 124;
const CONE_WEIGHT: f32 = 0.25;
const OVERDRAW_THRESHOLD: f32 = 1.05;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

/// Source files a glTF reads besides itself (external buffers), relative to its directory.
pub(crate) fn external_buffers(gltf_bytes: &[u8]) -> Result<Vec<String>, CookError> {
    let document =
        gltf::Gltf::from_slice(gltf_bytes).map_err(|error| CookError::Source(error.to_string()))?;
    Ok(document
        .buffers()
        .filter_map(|buffer| match buffer.source() {
            gltf::buffer::Source::Uri(uri) if !uri.starts_with("data:") => Some(uri.to_owned()),
            _ => None,
        })
        .collect())
}

/// Cook a glTF or GLB file into a meshlet mesh container.
pub(crate) fn cook(path: &Path) -> Result<Vec<u8>, CookError> {
    let source = |error: gltf::Error| CookError::Source(format!("{}: {error}", path.display()));
    let document = gltf::Gltf::open(path).map_err(source)?;
    let buffers =
        gltf::import_buffers(&document, path.parent(), document.blob.clone()).map_err(source)?;
    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next());
    let mut primitives = Vec::new();
    if let Some(scene) = scene {
        for node in scene.nodes() {
            gather(&node, IDENTITY, &buffers, &mut primitives)?;
        }
    }
    if primitives.is_empty() {
        return Err(CookError::Source(format!(
            "{}: no triangle primitives",
            path.display()
        )));
    }
    Ok(build(primitives))
}

type Matrix = [[f32; 4]; 4];
const IDENTITY: Matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|column| {
        std::array::from_fn(|row| (0..4).map(|k| a[k][row] * b[column][k]).sum())
    })
}

fn transform_point(m: &Matrix, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row])
}

fn transform_normal(m: &Matrix, n: [f32; 3]) -> [f32; 3] {
    // Inverse-transpose of the upper 3×3 via the cofactor matrix (scale is irrelevant after normalizing).
    let c = |r: usize, k: usize| m[k][r];
    let cofactor = [
        [
            c(1, 1) * c(2, 2) - c(1, 2) * c(2, 1),
            c(1, 2) * c(2, 0) - c(1, 0) * c(2, 2),
            c(1, 0) * c(2, 1) - c(1, 1) * c(2, 0),
        ],
        [
            c(0, 2) * c(2, 1) - c(0, 1) * c(2, 2),
            c(0, 0) * c(2, 2) - c(0, 2) * c(2, 0),
            c(0, 1) * c(2, 0) - c(0, 0) * c(2, 1),
        ],
        [
            c(0, 1) * c(1, 2) - c(0, 2) * c(1, 1),
            c(0, 2) * c(1, 0) - c(0, 0) * c(1, 2),
            c(0, 0) * c(1, 1) - c(0, 1) * c(1, 0),
        ],
    ];
    normalize(std::array::from_fn(|row| {
        (0..3).map(|k| cofactor[row][k] * n[k]).sum()
    }))
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length > 1e-20 {
        v.map(|x| x / length)
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn gather(
    node: &gltf::Node<'_>,
    parent: Matrix,
    buffers: &[gltf::buffer::Data],
    out: &mut Vec<(Vec<Vertex>, Vec<u32>)>,
) -> Result<(), CookError> {
    let world = multiply(&parent, &node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        for primitive in mesh
            .primitives()
            .filter(|p| p.mode() == gltf::mesh::Mode::Triangles)
        {
            let reader =
                primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| &data.0[..]));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<[f32; 3]> = positions.map(|p| transform_point(&world, p)).collect();
            let indices: Vec<u32> = match reader.read_indices() {
                Some(indices) => indices.into_u32().collect(),
                None => (0..u32::try_from(positions.len()).unwrap_or(u32::MAX)).collect(),
            };
            if indices
                .iter()
                .any(|&index| index as usize >= positions.len())
            {
                return Err(CookError::Source("glTF index out of range".into()));
            }
            let normals: Vec<[f32; 3]> = match reader.read_normals() {
                Some(normals) => normals.map(|n| transform_normal(&world, n)).collect(),
                None => smooth_normals(&positions, &indices),
            };
            let uvs: Vec<[f32; 2]> = reader.read_tex_coords(0).map_or_else(
                || vec![[0.0; 2]; positions.len()],
                |uv| uv.into_f32().collect(),
            );
            let vertices = positions
                .iter()
                .enumerate()
                .map(|(index, &position)| Vertex {
                    position,
                    normal: normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]),
                    uv: uvs.get(index).copied().unwrap_or_default(),
                })
                .collect();
            out.push((vertices, indices));
        }
    }
    for child in node.children() {
        gather(&child, world, buffers, out)?;
    }
    Ok(())
}

fn smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0_f32; 3]; positions.len()];
    for triangle in indices.as_chunks::<3>().0 {
        let [a, b, c] = [0, 1, 2].map(|corner| positions[triangle[corner] as usize]);
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let face = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        for &corner in triangle {
            for axis in 0..3 {
                normals[corner as usize][axis] += face[axis];
            }
        }
    }
    normals.into_iter().map(normalize).collect()
}

/// Octahedral unit-vector encoding (Cigolle et al. 2014) into signed normalized 16-bit.
pub(crate) fn octahedral(n: [f32; 3]) -> [i16; 2] {
    let sum = n[0].abs() + n[1].abs() + n[2].abs();
    let (mut x, mut y) = (n[0] / sum, n[1] / sum);
    if n[2] < 0.0 {
        let (ox, oy) = (x, y);
        x = (1.0 - oy.abs()) * ox.signum();
        y = (1.0 - ox.abs()) * oy.signum();
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "value is clamped to the i16 range before conversion"
    )]
    let quantize = |v: f32| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16;
    [quantize(x), quantize(y)]
}

struct Built {
    positions: Vec<[f32; 3]>,
    attributes: Vec<VertexAttributes>,
    indices: Vec<u32>,
    meshlets: Vec<Meshlet>,
    meshlet_vertices: Vec<u32>,
    meshlet_triangles: Vec<u8>,
    bounds: Vec<MeshletBounds>,
    submeshes: Vec<Submesh>,
}

fn build(primitives: Vec<(Vec<Vertex>, Vec<u32>)>) -> Vec<u8> {
    // Concatenate, then weld identical vertices across the whole mesh.
    let mut vertices = Vec::new();
    let mut ranges = Vec::new();
    let mut indices = Vec::new();
    for (primitive_vertices, primitive_indices) in primitives {
        let base = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
        vertices.extend(primitive_vertices);
        let start = indices.len();
        indices.extend(primitive_indices.into_iter().map(|index| index + base));
        ranges.push(start..indices.len());
    }
    let (unique, remap) = meshopt::generate_vertex_remap(&vertices, Some(&indices));
    let mut vertices = meshopt::remap_vertex_buffer(&vertices, unique, &remap);
    let mut indices = meshopt::remap_index_buffer(Some(&indices), unique, &remap);
    for range in &ranges {
        let submesh = &mut indices[range.clone()];
        meshopt::optimize_vertex_cache_in_place(submesh, vertices.len());
        let adapter = adapter(&vertices);
        meshopt::optimize_overdraw_in_place(submesh, &adapter, OVERDRAW_THRESHOLD);
    }
    meshopt::optimize_vertex_fetch_in_place(&mut indices, &mut vertices);

    let mut built = Built {
        positions: vertices.iter().map(|v| v.position).collect(),
        attributes: vertices
            .iter()
            .map(|v| VertexAttributes {
                normal_oct: octahedral(v.normal),
                uv: v.uv,
            })
            .collect(),
        indices: Vec::with_capacity(indices.len()),
        meshlets: Vec::new(),
        meshlet_vertices: Vec::new(),
        meshlet_triangles: Vec::new(),
        bounds: Vec::new(),
        submeshes: Vec::new(),
    };
    let adapter = adapter(&vertices);
    for range in ranges {
        let meshlets = meshopt::build_meshlets(
            &indices[range],
            &adapter,
            MESHLET_VERTICES,
            MESHLET_TRIANGLES,
            CONE_WEIGHT,
        );
        let index_offset = built.indices.len();
        let meshlet_offset = built.meshlets.len();
        for (record, meshlet) in meshlets.meshlets.iter().zip(meshlets.iter()) {
            let b = meshopt::compute_meshlet_bounds(meshlet, &adapter);
            built.bounds.push(MeshletBounds {
                center: b.center,
                radius: b.radius,
                cone_apex: b.cone_apex,
                cone_cutoff: b.cone_cutoff,
                cone_axis: b.cone_axis,
                reserved: 0.0,
            });
            built.meshlets.push(Meshlet {
                vertex_offset: u32::try_from(built.meshlet_vertices.len()).unwrap_or(u32::MAX),
                triangle_offset: u32::try_from(built.meshlet_triangles.len()).unwrap_or(u32::MAX),
                vertex_count: record.vertex_count,
                triangle_count: record.triangle_count,
            });
            built.indices.extend(
                meshlet
                    .triangles
                    .iter()
                    .map(|&local| meshlet.vertices[usize::from(local)]),
            );
            built.meshlet_vertices.extend_from_slice(meshlet.vertices);
            built.meshlet_triangles.extend_from_slice(meshlet.triangles);
        }
        built.submeshes.push(Submesh {
            index_offset: u32::try_from(index_offset).unwrap_or(u32::MAX),
            index_count: u32::try_from(built.indices.len() - index_offset).unwrap_or(u32::MAX),
            meshlet_offset: u32::try_from(meshlet_offset).unwrap_or(u32::MAX),
            meshlet_count: u32::try_from(built.meshlets.len() - meshlet_offset).unwrap_or(u32::MAX),
        });
    }
    encode(&built)
}

fn adapter(vertices: &[Vertex]) -> meshopt::VertexDataAdapter<'_> {
    meshopt::VertexDataAdapter::new(bytemuck::cast_slice(vertices), size_of::<Vertex>(), 0)
        .expect("Vertex stride and position offset are valid by construction")
}

fn encode(built: &Built) -> Vec<u8> {
    let mut bounds_min = [f32::INFINITY; 3];
    let mut bounds_max = [f32::NEG_INFINITY; 3];
    for position in &built.positions {
        for axis in 0..3 {
            bounds_min[axis] = bounds_min[axis].min(position[axis]);
            bounds_max[axis] = bounds_max[axis].max(position[axis]);
        }
    }
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let meta = MeshMeta {
        bounds_min,
        vertex_count: count(built.positions.len()),
        bounds_max,
        index_count: count(built.indices.len()),
        meshlet_count: count(built.meshlets.len()),
        max_meshlet_vertices: count(MESHLET_VERTICES),
        max_meshlet_triangles: count(MESHLET_TRIANGLES),
        submesh_count: count(built.submeshes.len()),
    };
    ContainerWriter::new(MESH_MAGIC)
        .section(mesh_tag::META, &[meta])
        .section(mesh_tag::POSITION, &built.positions)
        .section(mesh_tag::ATTRIBUTES, &built.attributes)
        .section(mesh_tag::INDICES, &built.indices)
        .section(mesh_tag::MESHLETS, &built.meshlets)
        .section(mesh_tag::MESHLET_VERTICES, &built.meshlet_vertices)
        .section(mesh_tag::MESHLET_TRIANGLES, &built.meshlet_triangles)
        .section(mesh_tag::MESHLET_BOUNDS, &built.bounds)
        .section(mesh_tag::SUBMESHES, &built.submeshes)
        .finish()
}
