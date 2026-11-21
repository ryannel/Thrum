//! Count the queue's upload bytes and exercise persistence across compute and dropped encoders.
use crate::support;
use std::fmt::Write as _;

fn record(h: &mut support::Harness, label: &str, changed: bool, queries: usize) {
    let bytes = h.sea.upload_bytes();
    assert_eq!(bytes.parameters, 176);
    assert_eq!(bytes.long, 192 * 32);
    assert_eq!(bytes.queries, queries as u64 * 8);
    assert_eq!(
        bytes.total,
        bytes.parameters + bytes.modes + bytes.long + bytes.queries
    );
    assert_eq!(
        bytes.modes,
        if changed {
            3 * u64::from(thrum_sea::PIXELS) * 32
        } else {
            0
        }
    );
    writeln!(h.record, "{label}: {bytes:?}").expect("record");
}

#[test]
fn still_changed_and_transition_upload_only_the_required_bytes() {
    let Some(mut h) = support::Harness::new(false) else {
        return;
    };
    let start = support::spectrum(5.0);
    let points = support::points();
    let first = h
        .frame(&start, 10.0, &points, true)
        .expect("submitted first frame");
    record(&mut h, "initial", true, points.len());
    let still = h
        .frame(&start, 10.0, &points, true)
        .expect("submitted still frame");
    record(&mut h, "still after transforms", false, points.len());
    assert_eq!(
        first.cascades, still.cascades,
        "transforms never overwrite modes"
    );
    assert_eq!(first.long, still.long);

    let mut removed = support::spectrum(5.0);
    removed.cascades[0] =
        thrum_sea_model::Cascade::new(removed.cascades[0].length(), &[]).expect("empty band");
    let removed_output = h
        .frame(&removed, 10.0, &points, true)
        .expect("in-place replacement");
    record(&mut h, "in-place band removal", true, points.len());
    for texel in removed_output.cascades[..thrum_sea::PIXELS as usize * 12]
        .as_chunks::<12>()
        .0
    {
        assert_eq!(&texel[..3], &[0.0; 3]);
        assert_eq!(texel[3], 1.0);
        assert_eq!(&texel[4..], &[0.0; 8]);
    }

    let changed = support::spectrum(7.0);
    let output = h
        .frame(&changed, 10.0, &points, true)
        .expect("changed spectrum");
    record(&mut h, "changed", true, points.len());
    support::check_cascades(&mut h, &output, &changed, 10.0, 8);
    // A discarded upload must not mark an upload as cached before its bytes can reach the GPU.
    h.frame(&start, 11.0, &points, false);
    record(&mut h, "changed then discarded", true, points.len());
    let recovered = h
        .frame(&start, 11.0, &points, true)
        .expect("frame after discard");
    record(&mut h, "same after discard", false, points.len());
    support::check_cascades(&mut h, &recovered, &start, 11.0, 8);

    let mut blended = support::spectrum(5.0);
    let transition =
        thrum_sea_model::Transition::new(start, changed, 10.0, 10.0).expect("transition");
    let addresses = storage(&blended);
    for time in [10.0, 10.001, 10.5, 12.0] {
        transition
            .at_into(time, &mut blended)
            .expect("reuse destination");
        assert_eq!(
            storage(&blended),
            addresses,
            "transition retains all allocations"
        );
        let output = h
            .frame(&blended, time, &points, true)
            .expect("transition frame");
        record(&mut h, "transition", time > 10.0, points.len());
        support::check_cascades(&mut h, &output, &blended, time, 8);
        let expected = transition.at(time).expect("snapshot compatibility");
        assert_eq!(blended.long, expected.long);
        for (a, b) in blended.cascades.iter().zip(expected.cascades) {
            assert_eq!(a.components(), b.components());
        }
    }
    let mut encoder = h
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    h.sea
        .upload(&h.queue, &mut encoder, &blended, 12.0, support::ANCHOR, &[])
        .expect("no queries");
    record(&mut h, "no queries", false, 0);
    let before = h.sea.upload_bytes();
    assert!(
        h.sea
            .upload(
                &h.queue,
                &mut encoder,
                &blended,
                f64::NAN,
                support::ANCHOR,
                &points
            )
            .is_err()
    );
    assert_eq!(
        before,
        h.sea.upload_bytes(),
        "invalid upload sends no bytes"
    );
    h.save("upload-bytes");
}

fn storage(s: &thrum_sea_model::Spectrum) -> [usize; 4] {
    [
        s.long.as_ptr() as usize,
        s.cascades[0].components().as_ptr() as usize,
        s.cascades[1].components().as_ptr() as usize,
        s.cascades[2].components().as_ptr() as usize,
    ]
}
