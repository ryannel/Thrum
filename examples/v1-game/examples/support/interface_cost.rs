//! CPU-side interface stage measurements in the Metal cost take.

pub(crate) fn stats(sample: &thrum_render::runtime::FrameSample) -> serde_json::Value {
    sample
        .interface
        .map_or(serde_json::Value::Null, |interface| {
            serde_json::json!({
                "upload_cpu_ns": interface.upload_cpu.as_nanos(),
                "encode_cpu_ns": interface.encode_cpu.as_nanos(),
                "uploaded_bytes": interface.uploaded_bytes,
                "dropped_rows": interface.dropped_rows,
                "extent_mismatch": interface.extent_mismatch,
            })
        })
}
