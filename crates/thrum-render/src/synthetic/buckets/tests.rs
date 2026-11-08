use super::{
    BucketError, BucketKey, BucketLimits, Buckets, MAX_BUCKETS, PADDING, PipelineClass,
    SEGMENT_ALIGN,
};
use crate::abi::InstanceRecord;
use crate::synthetic::mesh_arena::MeshId;
use crate::synthetic::upload::{InstanceUpload, PackedInstances};

fn key(mesh: MeshId, class: PipelineClass) -> BucketKey {
    BucketKey { mesh, class }
}

const LIMITS: BucketLimits = BucketLimits {
    max_storage_binding: 128 << 20,
    max_workgroups: 65_535,
};

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
    let mut buckets = Buckets::new(keys, slots as usize, LIMITS).expect("admitted");
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
    let mut buckets =
        Buckets::new([key(MeshId::CUBE, PipelineClass::Ordinary)], 4, LIMITS).expect("admitted");
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
    let mut buckets = Buckets::new(keys, count as usize, LIMITS).expect("admitted");
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

#[test]
fn admission_bounds_buckets_ranges_and_device_limits() {
    let ordinary = key(MeshId::CUBE, PipelineClass::Ordinary);
    assert_eq!(
        Buckets::new([], 4, LIMITS).err(),
        Some(BucketError::BucketCount)
    );
    let many = (0..=MAX_BUCKETS).map(|row| {
        key(
            MeshId::for_test(u32::try_from(row).expect("small")),
            PipelineClass::Ordinary,
        )
    });
    assert_eq!(
        Buckets::new(many, 4, LIMITS).err(),
        Some(BucketError::BucketCount)
    );
    // 1,000 slots in one bucket need at most 1,256 members (5,024 bytes) and
    // 4 workgroups of scratch (144 bytes).
    let tight = |bytes, groups| BucketLimits {
        max_storage_binding: bytes,
        max_workgroups: groups,
    };
    assert!(Buckets::new([ordinary], 1_000, tight(5_024, 4)).is_ok());
    // WebGPU's defaults admit the field's 100,513 slots in 64 buckets.
    let web = BucketLimits::of(&wgpu::Limits::default());
    assert!(
        Buckets::new(
            (0..64).map(|row| key(MeshId::for_test(row), PipelineClass::Ordinary)),
            100_513,
            web
        )
        .is_ok()
    );
    assert_eq!(
        Buckets::new([ordinary], 1_000, tight(5_020, 4)).err(),
        Some(BucketError::DeviceLimit)
    );
    assert_eq!(
        Buckets::new([ordinary], 1_000, tight(5_024, 3)).err(),
        Some(BucketError::DeviceLimit)
    );
}

#[test]
fn a_bad_mapping_leaves_the_live_layout_and_an_unchanged_one_is_reused() {
    let ordinary = key(MeshId::CUBE, PipelineClass::Ordinary);
    let mut buckets = Buckets::new([ordinary], 4, LIMITS).expect("admitted");
    for slot in 0..4 {
        buckets.set(slot, Some(ordinary)).expect("known");
    }
    assert_eq!(buckets.layout(&[3, 2, 1, 0]), Ok(true));
    let live = buckets.members().to_vec();
    assert_eq!(live[..4], [3, 2, 1, 0]);
    assert_eq!(buckets.layout(&[0, 1, 2]), Err(BucketError::Mapping));
    assert_eq!(buckets.layout(&[0, 1, 2, 4]), Err(BucketError::Mapping));
    assert_eq!(buckets.members(), live);
    // Same keys and mapping: nothing is rebuilt.
    assert_eq!(buckets.layout(&[3, 2, 1, 0]), Ok(false));
    // A new physical order, or a changed key, is.
    assert_eq!(buckets.layout(&[0, 1, 2, 3]), Ok(true));
    buckets.set(2, Some(ordinary)).expect("known");
    assert_eq!(
        buckets.layout(&[0, 1, 2, 3]),
        Ok(false),
        "same key is no change"
    );
    buckets.set(2, None).expect("known");
    assert_eq!(buckets.layout(&[0, 1, 2, 3]), Ok(true));
    assert_eq!(buckets.segments()[0].len, 3);
}
