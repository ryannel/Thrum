//! Reflection collected alongside the Metal source in the WGSL pipeline path.

use alloc::{string::String, vec::Vec};
use core::cell::RefCell;

const MAGIC: &[u8] = b"wgpu-metal-stage\x00\x01";

/// A Metal function and the host bindings required by its generated MSL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageRecord {
    pub function: String,
    pub workgroup_size: [u32; 3],
    pub workgroup_memory_sizes: Vec<u32>,
    pub sized_bindings: Vec<(naga::ResourceBinding, u32)>,
    pub immutable_buffer_mask: usize,
}

/// A stage's actual compiler input and the record used by the live pipeline.
#[derive(Debug)]
pub struct StageTranslation {
    pub stage: naga::ShaderStage,
    pub entry_point: String,
    pub source: String,
    /// The raw `MTLLanguageVersion` used by Metal's compile options.
    pub language_version: usize,
    pub preserve_invariance: bool,
    pub record: StageRecord,
}

std::thread_local! {
    static CAPTURE: RefCell<Option<Vec<StageTranslation>>> = const { RefCell::new(None) };
}

/// Observe stages translated by the calling thread during `create`.
/// Nested captures collect only the stages in their own scope.
pub fn capture_stage_translations<T>(create: impl FnOnce() -> T) -> (T, Vec<StageTranslation>) {
    struct Capture(Option<Vec<StageTranslation>>);
    impl Drop for Capture {
        fn drop(&mut self) {
            CAPTURE.with_borrow_mut(|current| *current = self.0.take());
        }
    }

    let _capture = Capture(CAPTURE.with_borrow_mut(|current| current.replace(Vec::new())));
    let value = create();
    let stages = CAPTURE.with_borrow_mut(|current| current.take().unwrap());
    (value, stages)
}

pub(super) fn observe(translation: impl FnOnce() -> StageTranslation) {
    CAPTURE.with_borrow_mut(|current| {
        if let Some(stages) = current {
            stages.push(translation());
        }
    });
}

fn put(output: &mut Vec<u8>, value: u32) {
    output.extend(value.to_le_bytes());
}

/// Prefix a Metal library with the records of the functions available to the caller.
pub fn encode_stage_records(records: &[StageRecord], metallib: &[u8]) -> Vec<u8> {
    let mut output = MAGIC.to_vec();
    put(&mut output, records.len().try_into().unwrap());
    for record in records {
        put(&mut output, record.function.len().try_into().unwrap());
        output.extend(record.function.as_bytes());
        for size in record.workgroup_size {
            put(&mut output, size);
        }
        put(
            &mut output,
            record.workgroup_memory_sizes.len().try_into().unwrap(),
        );
        for size in &record.workgroup_memory_sizes {
            put(&mut output, *size);
        }
        put(&mut output, record.sized_bindings.len().try_into().unwrap());
        for (binding, index) in &record.sized_bindings {
            put(&mut output, binding.group);
            put(&mut output, binding.binding);
            put(&mut output, *index);
        }
        output.extend((record.immutable_buffer_mask as u64).to_le_bytes());
    }
    output.extend(metallib);
    output
}

fn take<const N: usize>(input: &mut &[u8]) -> Option<[u8; N]> {
    let (word, rest) = input.split_first_chunk()?;
    *input = rest;
    Some(*word)
}

fn word(input: &mut &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(take(input)?))
}

fn list<T>(input: &mut &[u8], read: impl Fn(&mut &[u8]) -> Option<T>) -> Option<Vec<T>> {
    let len = word(input)?;
    (0..len).map(|_| read(input)).collect()
}

fn decode(input: &mut &[u8]) -> Option<StageRecord> {
    let len = word(input)? as usize;
    let function = core::str::from_utf8(input.split_off(..len)?).ok()?.into();
    let workgroup_size = [word(input)?, word(input)?, word(input)?];
    let workgroup_memory_sizes = list(input, word)?;
    let sized_bindings = list(input, |input| {
        let group = word(input)?;
        let binding = word(input)?;
        Some((naga::ResourceBinding { group, binding }, word(input)?))
    })?;
    let immutable_buffer_mask = u64::from_le_bytes(take(input)?).try_into().ok()?;
    Some(StageRecord {
        function,
        workgroup_size,
        workgroup_memory_sizes,
        sized_bindings,
        immutable_buffer_mask,
    })
}

/// Split cooked stage records from a library, or leave a plain library intact.
/// Returns an error if a record section is present but malformed.
pub fn split_stage_records(
    bytes: &[u8],
) -> Result<(Option<Vec<StageRecord>>, &[u8]), &'static str> {
    let Some(mut bytes) = bytes.strip_prefix(MAGIC) else {
        return Ok((None, bytes));
    };
    let records = list(&mut bytes, decode).ok_or("malformed Metal stage records")?;
    Ok((Some(records), bytes))
}
