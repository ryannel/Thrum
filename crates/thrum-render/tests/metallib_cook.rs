use std::{fs, io};

use thrum_test_support::Scratch;
use wgpu::hal::metal::{StageTranslation, encode_stage_records};

pub(crate) fn cook(translation: &StageTranslation, scratch: &Scratch) -> io::Result<Vec<u8>> {
    fs::write(scratch.join("source.metal"), &translation.source)?;
    fs::write(
        scratch.join("options.json"),
        format!(
            r#"{{"language_version":{},"preserve_invariance":{}}}"#,
            translation.language_version, translation.preserve_invariance,
        ),
    )?;
    fs::write(
        scratch.join("stage.record"),
        encode_stage_records(std::slice::from_ref(&translation.record), &[]),
    )?;
    let path = scratch.join("cooked.blob");
    thrum_cook::metallib::cook_stage(scratch.path(), &path)?;
    fs::read(path)
}
