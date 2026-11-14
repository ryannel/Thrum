//! A checked-in corrected-timeline slice is exercised by the workspace test gate.
#[test]
fn the_crowd_matches_the_retained_corrected_timeline() {
    let mut simulation = crowd_2d::Simulation::new(2000, 42);
    let result = crowd_2d::run_trace(&mut simulation, 240, false, |_| 0);
    let retained = include_str!("../retained/crowd.txt");
    let mut compared = 0;
    for line in retained
        .lines()
        .filter(|line| line.starts_with("slice 2000 "))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        let tick: usize = fields[2].parse().expect("tick");
        let expected = u64::from_str_radix(fields[3], 16).expect("digest");
        assert_eq!(result.timeline[tick - 1], expected, "tick {tick}");
        compared += 1;
    }
    assert_eq!(compared, 240);
    assert!(result.reused > 0 && result.hits > 0 && result.changed_rewinds > 0);
}
