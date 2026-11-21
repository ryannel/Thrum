//! Metal pass timestamps and actual allocation sizes for the cascade transforms.

use crate::support;

#[test]
fn cascade_passes() {
    let Some(mut harness) = support::Harness::new(false) else {
        return;
    };
    let resources = harness.sea.resource_bytes();
    for name in ["sea transform A", "sea transform B", "sea final transform"] {
        let (_, bytes) = resources
            .iter()
            .find(|(label, _)| label == name)
            .expect("work buffer");
        assert_eq!(*bytes, 3 * 6 * u64::from(thrum_sea::PIXELS) * 8, "{name}");
    }
    assert!(resources.contains(&(
        "sea transform twiddles".into(),
        u64::from(thrum_sea::TRANSFORM_SIZE / 2) * 8,
    )));
    let spectrum = support::spectrum(10.0);
    for frame in 0..24 {
        harness
            .frame(&spectrum, f64::from(frame), &[], true)
            .unwrap();
    }
    println!("{}", harness.record);
}
