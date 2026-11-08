use super::{
    MAX_PARTIAL_WRITES, PackedInstances, build_runs, needs_two_sided, pack_synthetic_instance,
};
use crate::{abi, synthetic::RenderError};

impl super::InstanceUpload {
    /// Physical row order and its packed rows, for layout witnesses.
    pub(in crate::synthetic) fn physical_rows(&self) -> (&[u32], &[u8]) {
        (&self.physical_to_slot, &self.packed)
    }
}

#[test]
fn changed_slots_form_bounded_contiguous_writes() {
    let mut runs = Vec::with_capacity(MAX_PARTIAL_WRITES);
    assert!(!build_runs(&[], &mut runs));
    assert!(runs.is_empty());
    assert!(!build_runs(&[0, 1, 2, 8], &mut runs));
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].source, 0..3);
    assert_eq!(runs[0].destination, 0);
    assert_eq!(runs[1].source, 3..4);
    assert_eq!(runs[1].destination, 8);
    let scattered: Vec<_> = (0..=MAX_PARTIAL_WRITES)
        .map(|row| u32::try_from(row * 2).expect("small test row"))
        .collect();
    assert!(build_runs(&scattered, &mut runs));
    assert_eq!(runs.len(), MAX_PARTIAL_WRITES);
}

#[test]
fn packed_rows_put_active_slots_first_without_changing_logical_identity() {
    let mut instances: Vec<_> = (0..4)
        .map(|slot| abi::InstanceRecord::identity(slot, 0))
        .collect();
    instances[1].ids[3] = 0;
    instances[3].ids[3] = 0;
    let packed = PackedInstances::new(&instances, 1).unwrap();
    assert_eq!(packed.active_count, 2);
    assert_eq!(packed.physical_to_slot, [0, 2, 1, 3]);
    assert_eq!(packed.slot_to_physical, [0, 2, 1, 3]);
    let (rows, remainder) = packed.bytes.as_chunks::<{ abi::INSTANCE_BYTES }>();
    assert_eq!(remainder, []);
    for (row, &logical) in rows.iter().zip(&packed.physical_to_slot) {
        assert_eq!(
            *row,
            pack_synthetic_instance(&instances[logical as usize], 1, logical).unwrap()
        );
    }
}

#[test]
fn reflected_or_degenerate_rows_need_the_two_sided_fallback() {
    let mut instance = abi::InstanceRecord::identity(0, 0);
    assert!(!needs_two_sided(&instance));
    instance.current[0][0] = -1.0;
    assert!(needs_two_sided(&instance));
    instance.current[0][0] = 0.0;
    assert!(needs_two_sided(&instance));
}

#[test]
fn shared_pack_validation_refuses_invalid_updates() {
    let mut instance = abi::InstanceRecord::identity(0, 0);
    assert!(pack_synthetic_instance(&instance, 1, 0).is_ok());
    instance.current[3][0] = f32::NAN;
    assert!(matches!(
        pack_synthetic_instance(&instance, 1, 0),
        Err(RenderError::InvalidLighting)
    ));
    instance.current[3][0] = 0.0;
    instance.ids[1] = 1;
    assert!(matches!(
        pack_synthetic_instance(&instance, 1, 0),
        Err(RenderError::Abi(abi::AbiError::MaterialIndex))
    ));
    let mut duplicate = [
        abi::InstanceRecord::identity(0, 0),
        abi::InstanceRecord::identity(1, 0),
    ];
    duplicate[1].ids[0] = 0;
    assert!(matches!(
        PackedInstances::new(&duplicate, 1),
        Err(RenderError::InvalidInstanceIdentity)
    ));
}
