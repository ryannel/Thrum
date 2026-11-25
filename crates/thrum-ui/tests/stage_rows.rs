//! Capacity and upload-byte witnesses on the frame host.

#[expect(
    dead_code,
    reason = "individual proof binaries select different fixture helpers"
)]
mod support;

use support::SDR;
use thrum_ui::{PaintListBuilder, SpriteRow};

#[test]
fn invalid_and_excess_rows_are_counted_and_unchanged_frames_write_no_bytes() {
    let Some(gpu) = support::device("interface paint rows", "portable") else {
        return;
    };
    let extent = [9, 7];
    let mut host = support::host(
        &gpu.capabilities,
        &gpu.queue,
        extent,
        SDR,
        Some(support::interface(2)),
        0.0,
    )
    .expect("prepare a bounded paint list");
    let view = support::drawable(&gpu.device, extent, SDR);
    let paint = |x| {
        let mut list = PaintListBuilder::new(extent);
        list.push(SpriteRow::new(0, [x, 0.0]));
        let mut invalid = SpriteRow::new(1, [0.0, 0.0]);
        invalid.tint[1] = f32::NAN;
        list.push(invalid);
        list.push(SpriteRow::new(1, [1.0, 0.0]));
        list.finish()
    };
    host.source_mut().paint = Some(paint(0.0));
    support::frame(&mut host, &view, extent);
    let first = support::sample(&mut host)
        .interface
        .expect("interface counters");
    assert_eq!(first.dropped_rows, 2);
    assert!(first.uploaded_bytes > 0);
    for _ in 0..100 {
        support::frame(&mut host, &view, extent);
        let stats = support::sample(&mut host)
            .interface
            .expect("interface counters");
        assert_eq!(stats.dropped_rows, 2);
        assert_eq!(stats.uploaded_bytes, 0);
    }
    host.source_mut().paint = Some(paint(1.0));
    support::frame(&mut host, &view, extent);
    let changed = support::sample(&mut host)
        .interface
        .expect("interface counters");
    assert!(changed.uploaded_bytes > 0);
    assert_eq!(changed.dropped_rows, 2);

    let mut excessive = support::interface(2);
    excessive.sheets.resize(9, excessive.sheets[0].clone());
    excessive.sheets[8].name = "ninth interface sheet".to_owned();
    let result = support::host(
        &gpu.capabilities,
        &gpu.queue,
        extent,
        SDR,
        Some(excessive),
        0.0,
    );
    let Err(error) = result else {
        panic!("a ninth sheet was accepted")
    };
    assert!(
        error.to_string().contains("ninth interface sheet"),
        "{error}"
    );
}
