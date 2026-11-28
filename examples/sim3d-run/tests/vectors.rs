const RETAINED: &str = include_str!("fixtures/pair-vectors.txt");

#[test]
fn retained_vectors_match() {
    let actual = sim3d_run::vector_rows();
    assert_eq!(first_difference(RETAINED, &actual), None);
    assert_eq!(
        actual
            .lines()
            .filter(|row| row.starts_with("seeded/"))
            .count(),
        5_000
    );
}

#[test]
fn changed_word_names_the_vector() {
    let mut rows = RETAINED.lines();
    let first = rows.next().expect("at least one retained vector");
    let (prefix, word) = first
        .rsplit_once(' ')
        .expect("the first row has a result word");
    let changed_word = u32::from_str_radix(word, 16).expect("binary32 word") ^ 1;
    let changed = format!(
        "{prefix} {changed_word:08x}\n{}",
        rows.collect::<Vec<_>>().join("\n")
    );
    let actual = sim3d_run::vector_rows();
    let id = first
        .split_ascii_whitespace()
        .next()
        .expect("the first row has an ID");
    assert_eq!(first_difference(&changed, &actual).as_deref(), Some(id));
}

fn first_difference(expected: &str, actual: &str) -> Option<String> {
    let mut expected = expected.lines();
    let mut actual = actual.lines();
    let mut number = 0;
    loop {
        number += 1;
        match (expected.next(), actual.next()) {
            (None, None) => return None,
            (a, b) if a == b => (),
            (a, b) => {
                let name = [a, b]
                    .into_iter()
                    .flatten()
                    .find_map(|row| row.split_ascii_whitespace().next());
                return Some(name.map_or_else(|| format!("row {number}"), str::to_owned));
            }
        }
    }
}
