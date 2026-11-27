use super::{Cell, Interaction, Patch, SampleSettings, Step, shader_constants};
use thrum_test_support::{record, records};

#[test]
fn interaction_records_match_the_shader_layout() {
    let step = shader_constants() + include_str!("../step.wgsl");
    let sample = Interaction::sample_source(0, 0, 1, true);
    let validate = |source: &str| {
        let shader = wgpu::naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&shader)
        .expect("portable interaction shader");
    };
    // Each shader also validates as a portable module, beyond the record check.
    validate(&step);
    validate(&sample);
    records::named(
        "interaction step",
        &step,
        &[
            record!(
                Step,
                shift,
                current,
                old_base,
                new_base,
                cell_base,
                region,
                source_first,
                source_count,
                depth_index,
                level,
                metres,
                depth,
                seconds,
                ticks,
            ),
            record!(Cell, region, low, high, blend, shore_loss),
            record!(Patch, place, axis, footprint, impulse, padding),
        ],
    );
    records::named(
        "interaction sample",
        &sample,
        &[record!(
            SampleSettings as "WakeSettings",
            origins,
            clocks,
            bases,
            fine_origins,
            fine_clock,
            fine_bases
        )],
    );
}
