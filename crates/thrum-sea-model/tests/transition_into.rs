//! Reused transition storage preserves snapshot semantics and refuses incompatible destinations.
use thrum_sea_model::{Forcing, Spectrum, Transition};

fn spectrum(wind_speed: f64) -> Spectrum {
    Spectrum::build(&Forcing {
        wind_speed,
        wind_direction: 0.41,
        fetch: 1e6,
        swells: vec![],
    })
    .expect("fixture")
}

fn equal(a: &Spectrum, b: &Spectrum) {
    assert_eq!(a.long, b.long);
    assert_eq!(
        a.tail_slope_variance.to_bits(),
        b.tail_slope_variance.to_bits()
    );
    for (a, b) in a.cascades.iter().zip(&b.cascades) {
        assert_eq!(a.length().to_bits(), b.length().to_bits());
        assert_eq!(a.components(), b.components());
    }
}

#[test]
fn repeated_and_out_of_order_times_reuse_storage_and_equal_snapshots() {
    let mut value = spectrum(5.0);
    let long = value.long.as_ptr();
    let cascades = value.cascades.each_ref().map(|c| c.components().as_ptr());
    let transition = Transition::new(spectrum(5.0), spectrum(7.0), 100.0, 10.0).expect("fixture");
    for time in [99.0, 100.0, 100.001, 101.0, 1000.0, 101.0, 99.0] {
        transition
            .at_into(time, &mut value)
            .expect("compatible destination");
        equal(&value, &transition.at(time).expect("snapshot"));
        assert_eq!(long, value.long.as_ptr());
        assert_eq!(
            cascades,
            value.cascades.each_ref().map(|c| c.components().as_ptr())
        );
    }
    let before = value.clone();
    assert!(transition.at_into(f64::NAN, &mut value).is_err());
    equal(&before, &value);
    value.long[0].phase += 1.0;
    let before = value.clone();
    assert!(transition.at_into(102.0, &mut value).is_err());
    equal(&before, &value);
    value.long.pop();
    let before = value.clone();
    assert!(transition.at_into(102.0, &mut value).is_err());
    equal(&before, &value);
}
