use crate::{Field, Profile, Type};

#[test]
fn cluster_extent_fits_each_mature_cell() {
    const DEPTH: [f64; 2] = [1450.0, 3800.0];
    // Subtracting a world-space centre from a puff's position can lose a
    // few ulps at the outer lattice sites.
    const ROUNDING_METRES: f64 = 1e-9;
    let weather = Profile {
        pitch: 4600.0,
        period: 1800.0,
        turret_pitch: 1300.0,
        turret_least: 0.22,
        turret_fade: 0.12,
        detail_mean: 0.5,
        erosion: 0.61,
    };
    let kind = Type {
        base: 1600.0,
        depth: DEPTH,
        liquid_water: 0.00032,
        droplet_radius: 12e-6,
        deck: 0.0,
        profile: [0.25, 0.47],
    };
    let field = Field::new(39, weather, vec![kind; 2]).expect("fair field");
    for i in -5..=5 {
        for j in -5..=5 {
            let cell = field.cell(i, j, 0);
            let depth = DEPTH[0] + cell.depth * (DEPTH[1] - DEPTH[0]);
            let cluster = field.cluster(cell, 0.5, 1.0, DEPTH, 300.0);
            assert!(!cluster.puffs.is_empty());
            for puff in cluster.puffs {
                let [x, y, z] = puff.centre;
                let reach = (x - cell.centre[0]).hypot(z - cell.centre[1]) + puff.radius;
                let top = y + puff.radius;
                assert!(
                    reach <= cell.radius + ROUNDING_METRES,
                    "{i}, {j}: {reach} m"
                );
                assert!(top <= depth + ROUNDING_METRES, "{i}, {j}: {top} m");
            }
        }
    }
}
