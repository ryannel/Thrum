// Standalone reproduction of the 3,588 archived independent rational vectors.
// Compile from the repository root with rustc 1.99.1 and pass extracted base/cases.tsv
// and extra.tsv from R0-F3-TRANSFORMED-BOUND-OPT-2026-11-03.tar.gz.
#[path = "../../../crates/thrum-source/src/render_bound.rs"]
mod render_bound;

fn main() {
    let mut count = 0;
    for path in std::env::args().skip(1) {
        let data = std::fs::read_to_string(path).unwrap();
        for line in data.lines() {
            let mut fields = line.split_whitespace();
            let name = fields.next().unwrap();
            let mut next = || f64::from_bits(u64::from_str_radix(fields.next().unwrap(), 16).unwrap());
            let q = std::array::from_fn(|_| next());
            let t = std::array::from_fn(|_| next());
            let s = std::array::from_fn(|_| next());
            let lo = std::array::from_fn(|_| next());
            let hi = std::array::from_fn(|_| next());
            let valid = fields.next().unwrap() == "1";
            let inside = fields.next().unwrap() == "1";
            let expected = if valid {
                Ok(inside)
            } else {
                Err(render_bound::TransformSourceError::InvalidQuaternion)
            };
            assert_eq!(
                render_bound::transformed_box_within_world(lo, hi, t, q, s),
                expected,
                "{name}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 3_588);
    println!("independent archived rational vectors matched: {count}");
}
