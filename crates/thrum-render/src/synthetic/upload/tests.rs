use super::{MAX_PARTIAL_WRITES, apply_to_baseline, build_runs, pack_synthetic_instance};
use crate::{abi, synthetic::RenderError};

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
fn packed_rows_update_only_named_baseline_slots() {
    let mut baseline = vec![0; 4 * abi::INSTANCE_BYTES];
    let staged = vec![7; 2 * abi::INSTANCE_BYTES];
    apply_to_baseline(&mut baseline, &staged, &[1, 3]);
    for (slot, row) in baseline
        .as_chunks::<{ abi::INSTANCE_BYTES }>()
        .0
        .iter()
        .enumerate()
    {
        assert!(
            row.iter()
                .all(|byte| *byte == if slot % 2 == 0 { 0 } else { 7 })
        );
    }
}

#[test]
fn shared_pack_validation_refuses_invalid_updates() {
    let mut instance = abi::InstanceRecord::identity(0, 0);
    assert!(pack_synthetic_instance(&instance, 1).is_ok());
    instance.current[3][0] = f32::NAN;
    assert!(matches!(
        pack_synthetic_instance(&instance, 1),
        Err(RenderError::InvalidLighting)
    ));
    instance.current[3][0] = 0.0;
    instance.ids[1] = 1;
    assert!(matches!(
        pack_synthetic_instance(&instance, 1),
        Err(RenderError::Abi(abi::AbiError::MaterialIndex))
    ));
}
