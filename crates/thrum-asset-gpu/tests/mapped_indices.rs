//! A mapped, multi-meshlet shipping product supplies the exact GPU index stream.
use std::sync::Arc;
use thrum_asset::{AssetStore, ContentHash, MeshProduct};
use thrum_asset_gpu::mesh_upload::MeshUpload;
use wgpu::util::DeviceExt;

fn product() -> Arc<MeshProduct> {
    let store =
        AssetStore::new(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data"));
    let hash =
        ContentHash::from_hex("f8cd990daedf3f166fb35b3ffcc8025805e14b7c7106188760ec9e1c67fa8638")
            .expect("hash");
    let bytes = store.map(&hash).expect("mapping");
    assert!(bytes.is_mapped());
    hash.verify(&bytes).expect("retained product identity");
    Arc::new(MeshProduct::open(bytes).expect("mesh"))
}

#[test]
fn mapped_section_is_retained_without_an_index_allocation() {
    let mesh = product();
    mesh.validate_references().expect("fixture deep validation");
    let upload = MeshUpload::new(Arc::clone(&mesh)).expect("coverage");
    assert_eq!(upload.indices().as_ptr(), mesh.indices().as_ptr());
    assert_eq!(
        upload.allocated_bytes(),
        mesh.meshlets().len() * size_of::<[u32; 2]>()
    );
    let mut first = 0;
    for (range, meshlet) in upload.ranges().iter().zip(mesh.meshlets()) {
        assert_eq!(*range, [first, meshlet.triangle_count * 3]);
        let corners =
            &mesh.meshlet_triangles()[meshlet.triangle_offset as usize..][..range[1] as usize];
        let expanded: Vec<_> = corners
            .iter()
            .map(|&i| mesh.meshlet_vertices()[meshlet.vertex_offset as usize + usize::from(i)])
            .collect();
        assert_eq!(
            &mesh.indices()[first as usize..][..expanded.len()],
            expanded
        );
        first += range[1];
    }
    assert_eq!(first as usize, mesh.indices().len());
    drop(mesh);
    assert_eq!(
        upload.indices().len(),
        672,
        "upload owns the mapping lifetime"
    );
}

#[test]
fn metal_receives_the_mapped_index_bytes() {
    let Some(thrum_test_support::Opened { device, queue, .. }) = thrum_test_support::open(
        "mapped mesh indices",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mesh = product();
    let upload = MeshUpload::new(Arc::clone(&mesh)).expect("coverage");
    let bytes = bytemuck::cast_slice(upload.indices());
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("mapped index source"),
        contents: bytes,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_SRC,
    });
    assert_eq!(
        thrum_test_support::readback::buffer(&device, &queue, &buffer, ..),
        bytemuck::cast_slice::<u32, u8>(mesh.indices())
    );
}
