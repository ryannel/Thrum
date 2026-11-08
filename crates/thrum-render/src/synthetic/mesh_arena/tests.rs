use super::{ArenaError, ArenaLimits, INDEX_BYTES, MeshArena, MeshId, MeshVertex, VERTEX_BYTES};

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
    assert_eq!(
        (
            reversed.base_vertex,
            reversed.vertex_count,
            reversed.first_index
        ),
        (0, 24, 36),
        "the reversed row shares the cube's vertices"
    );
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
        before,
        (arena.vertices.len(), arena.indices.len(), arena.rows.len())
    );
    assert_eq!(arena.row(MeshId::CUBE).expect("cube").vertex_count, 24);
}

#[test]
fn the_arena_refuses_growth_past_its_limits() {
    // Room for the cube rows (24 shared vertices, 72 indices) and one more
    // mesh of up to 48 vertices and 36 indices.
    let limits = ArenaLimits {
        vertices: 72,
        indices: 108,
    };
    let mut arena = MeshArena::new(limits).expect("arena");
    let (vertices, indices) = fan(12, 1.0);
    assert_eq!(vertices.len(), 14);
    let fits = arena.insert(&vertices, &indices).expect("fits");
    let row = *arena.row(fits).expect("row");
    // Vertices would fit (52 of 72); indices would not (144 of 108). The
    // refusal leaves the arena as it was.
    assert_eq!(arena.insert(&vertices, &indices), Err(ArenaError::Capacity));
    assert_eq!(*arena.row(fits).expect("row"), row);
    assert_eq!(arena.rows.len(), 3);
    let tight = ArenaLimits::for_binding(48 * 100);
    assert_eq!((tight.vertices, tight.indices), (100, 1200));
    assert_eq!(
        MeshArena::new(ArenaLimits {
            vertices: 24,
            indices: 71
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
fn the_packed_vertex_matches_the_shader_struct() {
    // Rust packing: position, normal, uv, tangent, little-endian f32s.
    let vertex = MeshVertex {
        position: [1.0, 2.0, 3.0],
        normal: [4.0, 5.0, 6.0],
        uv: [7.0, 8.0],
        tangent: [9.0, 10.0, 11.0, 12.0],
    };
    let mut bytes = Vec::new();
    vertex.pack(&mut bytes);
    assert_eq!(bytes.len() as u64, VERTEX_BYTES);
    let floats: Vec<f32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&word| f32::from_le_bytes(word))
        .collect();
    assert_eq!(floats, (1..=12).map(|v| v as f32).collect::<Vec<_>>());
    // The shader reads position as a.xyz, normal as (a.w, b.xy), uv as b.zw
    // and tangent as c, so its struct must be three tightly packed vec4s.
    let source = include_str!("../../synthetic.wgsl");
    let declaration = source
        .lines()
        .find(|line| line.starts_with("struct MeshVertex"))
        .expect("synthetic.wgsl declares MeshVertex");
    let module = wgpu::naga::front::wgsl::parse_str(declaration).expect("parses");
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some("MeshVertex"))
        .expect("struct");
    let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("MeshVertex is a struct");
    };
    assert_eq!(u64::from(*span), VERTEX_BYTES);
    assert_eq!(
        members.iter().map(|m| m.offset).collect::<Vec<_>>(),
        [0, 16, 32]
    );
    assert!(source.contains("let local = vertex.a.xyz;"));
    assert!(source.contains("let n = vec3(vertex.a.w, vertex.b.xy);"));
    assert!(source.contains("output.uv = vertex.b.zw;"));
}

#[test]
fn base_vertices_stay_within_a_draw() {
    let limits = ArenaLimits::for_binding(u64::MAX);
    assert_eq!(limits.vertices, i32::MAX.unsigned_abs());
    let arena = MeshArena::new(ROOMY).expect("cube");
    for row in arena.rows() {
        assert_eq!(row.draw_base_vertex(), Ok(0));
    }
    let far = super::MeshRow {
        base_vertex: u32::MAX,
        ..arena.rows()[0]
    };
    assert_eq!(far.draw_base_vertex(), Err(ArenaError::Capacity));
}
