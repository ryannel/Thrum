//! M1.7: a river file holds a solve's output and decodes to the same bytes; its key follows its
//! inputs; a stale file is refused by name; the reader lives in `thrum-water-model`, which does
//! not depend on the solver.
use crate::support;
use support::valley;
use thrum_river_model::{Manning, Profile, SOLVER_VERSION, key, key_for_version, solve, write};
use thrum_water_model::{Bed, RiverFile};

#[test]
fn a_river_file_decodes_to_the_same_bytes_and_the_same_chart() {
    let (grid, declaration) = support::section::short_reach();
    let profile = Profile::default();
    let s = solve(&grid, &declaration, &profile).unwrap();
    let k = key(&grid, &declaration, &profile);
    let file = s.file(k).unwrap();
    let bytes = write(&file);
    let read = RiverFile::read(&bytes, &k).unwrap();
    assert_eq!(read, file);
    assert_eq!(write(&read), bytes);
    println!("{} bytes for {} cells", bytes.len(), grid.heights().len());
    let bed = Bed::new(
        grid.origin(),
        grid.spacing(),
        grid.size(),
        grid.heights().to_vec(),
    )
    .unwrap();
    let body = |name: &str| (name == declaration.receivers[0].name).then_some(0);
    let from_file = read.reaches[0].chart(&bed, body).unwrap();
    let from_solve = s.chart(&bed, body).unwrap();
    assert_eq!(from_file.levels, from_solve.levels);
    assert_eq!(from_file.currents, from_solve.currents);
    assert_eq!(from_file.footprint, from_solve.footprint);
    assert_eq!(read.reaches[0].loss, s.reach().unwrap().loss);

    // A file whose key does not match its inputs is refused by name.
    let mut other = k;
    other[0] ^= 1;
    let err = RiverFile::read(&bytes, &other).unwrap_err();
    println!("{}: {}", err.name, err.reason);
    assert_eq!(err.name, "river file");
    assert!(err.reason.contains("key"));
}

#[test]
fn the_key_changes_with_the_bed_the_discharge_the_roughness_and_the_version() {
    let (grid, declaration) = valley::valley(0.25);
    let profile = Profile::default();
    let k = key(&grid, &declaration, &profile);
    assert_eq!(k, key(&grid, &declaration, &profile));
    assert_eq!(
        k,
        key_for_version(&grid, &declaration, &profile, SOLVER_VERSION)
    );
    let mut heights = grid.heights().to_vec();
    heights[4000] += 0.001;
    let moved =
        thrum_river_model::Grid::new(grid.origin(), grid.spacing(), grid.size(), heights).unwrap();
    assert_ne!(k, key(&moved, &declaration, &profile));
    let mut more = declaration.clone();
    more.springs[0].discharge *= 1.01;
    assert_ne!(k, key(&grid, &more, &profile));
    let mut rough = declaration.clone();
    rough.manning = Manning::Uniform(0.041);
    assert_ne!(k, key(&grid, &rough, &profile));
    assert_ne!(
        k,
        key_for_version(&grid, &declaration, &profile, SOLVER_VERSION + 1)
    );
    // The thread count never changes a result, so it is not in the key.
    assert_eq!(
        k,
        key(
            &grid,
            &declaration,
            &Profile {
                threads: 3,
                ..profile
            }
        )
    );
}

#[test]
fn the_reader_crate_does_not_depend_on_the_solver() {
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../thrum-water-model/Cargo.toml");
    let text = std::fs::read_to_string(manifest).unwrap();
    assert!(!text.contains("thrum-river-model"));
}
