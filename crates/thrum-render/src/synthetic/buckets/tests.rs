use super::{BucketError, BucketKey, Buckets, PADDING, PipelineClass, SEGMENT_ALIGN};
use crate::abi::InstanceRecord;
use crate::synthetic::mesh_arena::MeshId;
use crate::synthetic::upload::{InstanceUpload, PackedInstances};

fn key(mesh: MeshId, class: PipelineClass) -> BucketKey {
    BucketKey { mesh, class }
}

fn instance(slot: u32, mirror: f32) -> InstanceRecord {
    let mut instance = InstanceRecord::identity(slot, 0);
    instance.current[0][0] = mirror;
    instance
}

#[test]
fn the_determinant_picks_the_pipeline_class() {
    assert_eq!(
        PipelineClass::of(&instance(0, 1.0)),
        PipelineClass::Ordinary
    );
    assert_eq!(
        PipelineClass::of(&instance(0, -1.0)),
        PipelineClass::Mirrored
    );
    assert_eq!(
        PipelineClass::of(&instance(0, 0.0)),
        PipelineClass::TwoSided
    );
    // Tiny but valid scales stay ordinary; the threshold is the upload's.
    assert_eq!(
        PipelineClass::of(&instance(0, 1e-9)),
        PipelineClass::Ordinary
    );
    assert_eq!(
        PipelineClass::of(&instance(0, -1e-13)),
        PipelineClass::TwoSided
    );
}

#[test]
fn segments_are_aligned_ordered_and_padded() {
    let keys = [
        key(MeshId::CUBE, PipelineClass::Mirrored),
        key(MeshId::CUBE, PipelineClass::Ordinary),
        key(MeshId::CUBE_REVERSED, PipelineClass::Ordinary),
        key(MeshId::CUBE, PipelineClass::Ordinary),
    ];
    let slots = 600_u32;
    let mut buckets = Buckets::new(keys, slots as usize);
    // 300 ordinary cubes, 1 mirrored cube, no reversed cubes; the rest inactive.
    for slot in 0..300 {
        buckets
            .set(slot * 2, Some(key(MeshId::CUBE, PipelineClass::Ordinary)))
            .expect("known");
    }
    buckets
        .set(7, Some(key(MeshId::CUBE, PipelineClass::Mirrored)))
        .expect("known");
    let identity: Vec<u32> = (0..slots).collect();
    buckets.layout(&identity).expect("layout");
    let segments = buckets.segments();
    assert_eq!(segments.len(), 3, "duplicate keys collapse");
    assert_eq!(
        segments
            .iter()
            .map(|s| (s.key, s.start, s.len, s.capacity))
            .collect::<Vec<_>>(),
        [
            (key(MeshId::CUBE, PipelineClass::Ordinary), 0, 300, 512),
            (key(MeshId::CUBE, PipelineClass::Mirrored), 512, 1, 256),
            (
                key(MeshId::CUBE_REVERSED, PipelineClass::Ordinary),
                768,
                0,
                256
            ),
        ]
    );
    let members = buckets.members();
    assert_eq!(members.len(), 1024);
    for segment in segments {
        assert_eq!(segment.start % SEGMENT_ALIGN, 0);
        let (start, len) = (segment.start as usize, segment.len as usize);
        let end = start + segment.capacity as usize;
        assert!(members[start + len..end].iter().all(|&m| m == PADDING));
        assert!(members[start..start + len].iter().all(|&m| m != PADDING));
        // Logical-slot order within the bucket.
        assert!(members[start..start + len].windows(2).all(|w| w[0] < w[1]));
    }
    assert_eq!(members[512], 7);
}

#[test]
fn unknown_keys_are_refused_and_leave_the_slot() {
    let mut buckets = Buckets::new([key(MeshId::CUBE, PipelineClass::Ordinary)], 4);
    buckets
        .set(1, Some(key(MeshId::CUBE, PipelineClass::Ordinary)))
        .expect("known");
    assert_eq!(
        buckets.set(1, Some(key(MeshId::CUBE, PipelineClass::Mirrored))),
        Err(BucketError::UnknownBucket)
    );
    assert_eq!(buckets.set(9, None), Err(BucketError::Capacity));
    buckets.layout(&[0, 1, 2, 3]).expect("layout");
    assert_eq!(buckets.segments()[0].len, 1);
    assert_eq!(buckets.members()[0], 1);
}

#[test]
fn members_follow_upload_activity_churn() {
    let (device, queue) = pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter");
        adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device")
    });
    let count = 96_u32;
    let mesh_of = |slot: u32| {
        if slot.is_multiple_of(3) {
            MeshId::CUBE_REVERSED
        } else {
            MeshId::CUBE
        }
    };
    let mut instances: Vec<_> = (0..count)
        .map(|slot| instance(slot, if slot % 5 == 0 { -1.0 } else { 1.0 }))
        .collect();
    for slot in (0..count).step_by(4) {
        instances[slot as usize].ids[3] = 0;
    }
    let keys = [MeshId::CUBE, MeshId::CUBE_REVERSED]
        .into_iter()
        .flat_map(|mesh| {
            [PipelineClass::Ordinary, PipelineClass::Mirrored].map(|class| key(mesh, class))
        });
    let mut buckets = Buckets::new(keys, count as usize);
    let packed = PackedInstances::new(&instances, 1).expect("pack");
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bucket test instances"),
        size: u64::from(count) * crate::abi::INSTANCE_BYTES as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut upload = InstanceUpload::new(buffer, packed, &instances, 1, count);
    for frame in 0..5_u32 {
        if frame > 0 {
            let changed: Vec<u32> = (0..count).filter(|s| (s + frame) % 3 == 0).collect();
            for &slot in &changed {
                let instance = &mut instances[slot as usize];
                instance.ids[3] = u32::from(instance.ids[3] == 0);
            }
            upload
                .replace(&queue, &instances, &changed, |_| false)
                .expect("replace");
        }
        for (slot, instance) in (0..count).zip(&instances) {
            let key =
                (instance.ids[3] != 0).then(|| key(mesh_of(slot), PipelineClass::of(instance)));
            buckets.set(slot, key).expect("known");
        }
        let (physical_to_slot, _) = upload.physical_rows();
        let mut slot_to_physical = vec![0; count as usize];
        for (physical, &slot) in (0..count).zip(physical_to_slot) {
            slot_to_physical[slot as usize] = physical;
        }
        buckets.layout(&slot_to_physical).expect("layout");
        // Every active slot appears exactly once, in its own bucket, as its
        // current physical row, and every member is an active row.
        let mut seen = vec![false; count as usize];
        let active = upload.active_count();
        for segment in buckets.segments() {
            let start = segment.start as usize;
            for &physical in &buckets.members()[start..start + segment.len as usize] {
                assert!(physical < active, "frame {frame}: inactive row {physical}");
                let slot = physical_to_slot[physical as usize];
                let instance = &instances[slot as usize];
                assert_eq!(segment.key.mesh, mesh_of(slot));
                assert_eq!(segment.key.class, PipelineClass::of(instance));
                assert!(!std::mem::replace(&mut seen[slot as usize], true));
            }
        }
        for (slot, instance) in instances.iter().enumerate() {
            assert_eq!(
                seen[slot],
                instance.ids[3] != 0,
                "frame {frame} slot {slot}"
            );
        }
    }
}
