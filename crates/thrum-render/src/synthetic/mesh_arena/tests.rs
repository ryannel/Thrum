use super::{
    ArenaError, ArenaLimits, INDEX_BYTES, MeshArena, MeshId, MeshVertex, VERTEX_BYTES,
    physical_meshes,
};
use crate::abi;
use crate::synthetic::upload::{InstanceUpload, PackedInstances, pack_synthetic_instance};

const ROOMY: ArenaLimits = ArenaLimits {
    vertices: 1 << 16,
    indices: 1 << 18,
};

fn vertex(position: [f32; 3]) -> MeshVertex {
    MeshVertex {
        position,
        normal: [0.0, 1.0, 0.0],
        uv: [0.0, 0.0],
        tangent: [1.0, 0.0, 0.0, 1.0],
    }
}

/// A fan of `triangles` around the origin, with radius `scale`.
fn fan(triangles: u32, scale: f32) -> (Vec<MeshVertex>, Vec<u32>) {
    let mut vertices = vec![vertex([0.0, 0.0, 0.0])];
    for step in 0..=triangles {
        #[expect(clippy::cast_precision_loss, reason = "small test count")]
        let angle = step as f32 / triangles as f32 * std::f32::consts::TAU;
        vertices.push(vertex([angle.cos() * scale, 0.0, angle.sin() * scale]));
    }
    let indices = (0..triangles)
        .flat_map(|step| [0, step + 1, step + 2])
        .collect();
    (vertices, indices)
}

fn device() -> (wgpu::Device, wgpu::Queue) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter");
        adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device")
    })
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer, bytes: u64) -> Vec<u8> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("arena test readback"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, bytes);
    queue.submit([encoder.finish()]);
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.expect("map"));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("poll");
    staging
        .slice(..)
        .get_mapped_range()
        .expect("mapped")
        .to_vec()
}

/// The GPU arrays hold every row's own vertices and indices at its offsets.
fn assert_gpu_matches(arena: &MeshArena, device: &wgpu::Device, queue: &wgpu::Queue) {
    let gpu = arena.buffers().expect("uploaded");
    let vertices = read(
        device,
        queue,
        &gpu.vertices,
        arena.vertices.len() as u64 * VERTEX_BYTES,
    );
    let indices = read(
        device,
        queue,
        &gpu.indices,
        arena.indices.len() as u64 * INDEX_BYTES,
    );
    for row in arena.rows() {
        let mut expected = Vec::new();
        for vertex in &arena.vertices[row.vertices()] {
            vertex.pack(&mut expected);
        }
        let start = row.base_vertex as usize * 48;
        assert_eq!(vertices[start..start + expected.len()], expected, "{row:?}");
        let expected: Vec<u8> = arena.indices[row.indices()]
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect();
        let start = row.first_index as usize * 4;
        assert_eq!(indices[start..start + expected.len()], expected, "{row:?}");
    }
}

#[test]
fn the_cube_rows_match_the_shader_cube() {
    let arena = MeshArena::new(ROOMY).expect("arena");
    let cube = *arena.row(MeshId::CUBE).expect("cube");
    let reversed = *arena.row(MeshId::CUBE_REVERSED).expect("reversed");
    assert_eq!((cube.vertex_count, cube.index_count), (24, 36));
    assert_eq!(cube.bounds, [[-1.0; 3], [1.0; 3]]);
    assert_eq!(reversed.bounds, cube.bounds);
    // Face 0 corner 3 of `synthetic.wgsl`: n + t*1 + cross(n,t)*1 = (1, -1, -1).
    let corner = arena.vertices[cube.base_vertex as usize + 3];
    assert_eq!(corner.position, [1.0, -1.0, -1.0]);
    assert_eq!(corner.uv, [1.0, 1.0]);
    // Every cube triangle faces along its vertex normal; every reversed one
    // faces against it.
    for (row, sign) in [(cube, 1.0), (reversed, -1.0)] {
        for triangle in arena.indices[row.indices()].as_chunks::<3>().0 {
            let [a, b, c] = std::array::from_fn(|corner| {
                arena.vertices[(row.base_vertex + triangle[corner]) as usize]
            });
            let e1: [f32; 3] = std::array::from_fn(|i| b.position[i] - a.position[i]);
            let e2: [f32; 3] = std::array::from_fn(|i| c.position[i] - a.position[i]);
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let facing: f32 = (0..3).map(|i| cross[i] * a.normal[i]).sum();
            assert!(facing * sign > 0.0, "{triangle:?} winding");
        }
    }
}

#[test]
fn malformed_meshes_are_refused_before_anything_is_written() {
    let mut arena = MeshArena::new(ROOMY).expect("arena");
    let (vertices, mut indices) = fan(4, 1.0);
    let before = (arena.vertices.len(), arena.indices.len(), arena.rows.len());
    let vertex_count = u32::try_from(vertices.len()).expect("small");
    indices[5] = vertex_count;
    assert_eq!(
        arena.insert(&vertices, &indices),
        Err(ArenaError::IndexOutOfRange {
            index: vertex_count,
            vertex_count
        })
    );
    assert_eq!(
        arena.insert(&vertices, &indices[..4]),
        Err(ArenaError::NotTriangles)
    );
    assert_eq!(arena.insert(&vertices, &[]), Err(ArenaError::NotTriangles));
    let (mut vertices, indices) = fan(4, 1.0);
    vertices[2].tangent[3] = f32::NAN;
    assert_eq!(
        arena.insert(&vertices, &indices),
        Err(ArenaError::NonFinite)
    );
    assert_eq!(
        arena.replace(MeshId(9), &vertices, &indices),
        Err(ArenaError::UnknownMesh)
    );
    assert_eq!(
        arena.replace(MeshId::CUBE, &vertices, &indices),
        Err(ArenaError::NonFinite)
    );
    assert_eq!(
        before,
        (arena.vertices.len(), arena.indices.len(), arena.rows.len())
    );
    assert_eq!(arena.row(MeshId::CUBE).expect("cube").vertex_count, 24);
}

#[test]
fn the_arena_refuses_growth_past_its_limits() {
    // Room for the two cubes and one more 24-vertex, 36-index mesh.
    let limits = ArenaLimits {
        vertices: 72,
        indices: 108,
    };
    let mut arena = MeshArena::new(limits).expect("arena");
    let (vertices, indices) = fan(12, 1.0);
    assert_eq!(vertices.len(), 14);
    let fits = arena.insert(&vertices, &indices).expect("fits");
    assert_eq!(arena.insert(&vertices, &indices), Err(ArenaError::Capacity));
    // A replacement that must move past the limit is refused and leaves the
    // row as it was.
    let (larger, larger_indices) = fan(30, 1.0);
    let row = *arena.row(fits).expect("row");
    assert_eq!(
        arena.replace(fits, &larger, &larger_indices),
        Err(ArenaError::Capacity)
    );
    assert_eq!(*arena.row(fits).expect("row"), row);
    let tight = ArenaLimits::for_binding(48 * 100);
    assert_eq!((tight.vertices, tight.indices), (100, 1200));
    assert_eq!(
        MeshArena::new(ArenaLimits {
            vertices: 47,
            indices: 1000
        })
        .err(),
        Some(ArenaError::Capacity)
    );
}

#[test]
fn bounds_come_from_the_referenced_vertices() {
    let mut arena = MeshArena::new(ROOMY).expect("arena");
    let (mut vertices, indices) = fan(6, 2.0);
    // An unreferenced vertex far away does not widen the bounds.
    vertices.push(vertex([100.0, 100.0, 100.0]));
    let mesh = arena.insert(&vertices, &indices).expect("insert");
    let bounds = arena.row(mesh).expect("row").bounds;
    assert_eq!(bounds[0][1], 0.0);
    assert_eq!(bounds[1][1], 0.0);
    assert!((bounds[1][0] - 2.0).abs() < 1e-6 && (bounds[0][0] + 2.0).abs() < 1e-6);
}

#[test]
fn growth_keeps_every_row_and_its_offsets() {
    let (device, queue) = device();
    let mut arena = MeshArena::new(ROOMY).expect("arena");
    let first = arena.upload(&device, &queue);
    assert!(first.reallocated);
    let mut meshes = vec![MeshId::CUBE, MeshId::CUBE_REVERSED];
    let mut reallocations = 0;
    for count in [200_u32, 900, 2000, 3000] {
        let before: Vec<_> = meshes
            .iter()
            .map(|&mesh| *arena.row(mesh).expect("row"))
            .collect();
        let (vertices, indices) = fan(count, 1.0);
        meshes.push(arena.insert(&vertices, &indices).expect("insert"));
        let upload = arena.upload(&device, &queue);
        reallocations += usize::from(upload.reallocated);
        for (mesh, row) in meshes.iter().zip(&before) {
            assert_eq!(arena.row(*mesh).expect("row"), row, "{mesh:?} moved");
        }
        assert_gpu_matches(&arena, &device, &queue);
    }
    assert!(reallocations >= 2, "the run grows the buffers");
    assert_eq!(
        meshes.iter().map(|mesh| mesh.row()).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5]
    );
    // An upload with nothing new writes nothing.
    assert_eq!(
        arena.upload(&device, &queue),
        super::ArenaUpload {
            reallocated: false,
            bytes: 0
        }
    );
}

#[test]
fn replacement_keeps_the_mesh_identity() {
    let (device, queue) = device();
    let mut arena = MeshArena::new(ROOMY).expect("arena");
    let (vertices, indices) = fan(64, 1.0);
    let a = arena.insert(&vertices, &indices).expect("a");
    let b = arena.insert(&vertices, &indices).expect("b");
    arena.upload(&device, &queue);
    let (old_a, old_b) = (*arena.row(a).expect("a"), *arena.row(b).expect("b"));
    // Smaller: rewritten in place.
    let (smaller, smaller_indices) = fan(16, 3.0);
    arena
        .replace(a, &smaller, &smaller_indices)
        .expect("smaller");
    let row = *arena.row(a).expect("a");
    assert_eq!(
        (row.base_vertex, row.first_index),
        (old_a.base_vertex, old_a.first_index)
    );
    assert_eq!((row.vertex_count, row.index_count), (18, 48));
    assert!((row.bounds[1][0] - 3.0).abs() < 1e-6);
    assert_eq!(arena.unused(), (66 - 18, 192 - 48));
    let upload = arena.upload(&device, &queue);
    assert!(!upload.reallocated);
    assert_eq!(upload.bytes, 18 * VERTEX_BYTES + 48 * INDEX_BYTES);
    assert_gpu_matches(&arena, &device, &queue);
    // Larger: moves to the end under the same identity; b is untouched.
    let (larger, larger_indices) = fan(500, 0.5);
    arena.replace(a, &larger, &larger_indices).expect("larger");
    let row = *arena.row(a).expect("a");
    assert!(row.base_vertex > old_b.base_vertex && row.first_index > old_b.first_index);
    assert_eq!(*arena.row(b).expect("b"), old_b);
    assert_eq!(arena.unused(), (66 - 18 + 18, 192 - 48 + 48));
    arena.upload(&device, &queue);
    assert_gpu_matches(&arena, &device, &queue);
    assert_eq!(arena.rows().len(), 4);
}

#[test]
fn culling_churn_keeps_each_physical_row_on_its_slots_mesh() {
    let (device, queue) = device();
    let count = 64_u32;
    let mut instances: Vec<_> = (0..count)
        .map(|slot| abi::InstanceRecord::identity(slot, 0))
        .collect();
    for slot in (0..count).step_by(3) {
        instances[slot as usize].ids[3] = 0;
    }
    let slot_meshes: Vec<_> = (0..count).map(|slot| MeshId(2 + slot % 5)).collect();
    let packed = PackedInstances::new(&instances, 1).expect("pack");
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("arena test instances"),
        size: u64::from(count) * abi::INSTANCE_BYTES as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut upload = InstanceUpload::new(buffer, packed, &instances, 1, count);
    let check = |upload: &InstanceUpload, instances: &[abi::InstanceRecord]| {
        let (physical_to_slot, packed) = upload.physical_rows();
        let table = physical_meshes(physical_to_slot, &slot_meshes);
        let (rows, _) = packed.as_chunks::<{ abi::INSTANCE_BYTES }>();
        let active = upload.active_count() as usize;
        for (physical, (&slot, row)) in physical_to_slot.iter().zip(rows).enumerate() {
            // The row holds the slot's own instance, and the table names the
            // slot's own mesh.
            assert_eq!(
                *row,
                pack_synthetic_instance(&instances[slot as usize], 1, slot).expect("pack")
            );
            assert_eq!(table[physical], slot_meshes[slot as usize].row());
            assert_eq!(physical < active, instances[slot as usize].ids[3] != 0);
        }
    };
    check(&upload, &instances);
    // Deactivate, reactivate and move slots over several frames.
    for frame in 0..6_u32 {
        let changed: Vec<u32> = (0..count).filter(|slot| (slot + frame) % 4 == 0).collect();
        for &slot in &changed {
            let instance = &mut instances[slot as usize];
            instance.ids[3] = u32::from(instance.ids[3] == 0);
            instance.current[3][0] += 1.0;
        }
        upload
            .replace(&queue, &instances, &changed, |_| false)
            .expect("replace");
        check(&upload, &instances);
    }
}
