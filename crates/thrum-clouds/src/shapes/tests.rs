use super::*;
use thrum_cloud_model::{Footprint, HashedShapes, Parameters, PlacedSlab, Profile};

#[test]
fn march_heights_enclose_scaled_hash_selected_columns() {
    let slab = PlacedSlab {
        extent: 1_600.0,
        floor: -300.0,
        heading: 0.0,
        footprint: Arc::new(
            Footprint::from_columns([4; 2], 400.0, vec![[200.0, -300.0, 1_300.0]; 16])
                .expect("volume footprint"),
        ),
    };
    let library = Arc::new(PlacedLibrary::new(vec![slab], vec![]).expect("library"));
    let config = ShapesConfig {
        library: Arc::clone(&library),
        lease: ProductLease::default(),
        range: [8_000.0, 16_000.0],
        capacity: 16,
    };
    let kind = Type {
        base: 1_600.0,
        depth: [220.0, 950.0],
        liquid_water: 0.0003,
        droplet_radius: 9e-6,
        deck: 0.0,
        profile: [0.2, 0.3],
    };
    let profile = Profile {
        pitch: 4_800.0,
        turret_pitch: 340.0,
        turret_least: 0.18,
        turret_fade: 0.12,
        period: 1_600.0,
        detail_mean: 0.5,
        erosion: 0.0,
    };
    let model = Field::new(230, profile, vec![kind; 2]).expect("weather field");
    let mut state = State::new(
        Parameters {
            coverage: 0.5,
            kind: 1.0,
            wind: [0.0; 2],
            time_scale: 1.0,
        },
        5.0,
        16,
    )
    .expect("weather state");
    state
        .set_hashed_library(
            library,
            HashedShapes {
                block_sites: 3,
                shares: vec![1.0; 2],
            },
        )
        .expect("hash placement");
    let [bottom, top] = config.heights(
        &state,
        kind,
        profile.pitch,
        [kind.base, kind.base + kind.depth[1]],
    );

    let mut beyond_unscaled = false;
    for z in -3..=3 {
        for x in -3..=3 {
            let Some((shape, _)) = model.hashed_shape([x, z], &state) else {
                continue;
            };
            let [x, z] = shape.centre;
            let column = model.query([x, 0.0, z], &state);
            assert!(column.optical_depth > 0.0);
            assert!(bottom <= column.base, "{bottom} > {}", column.base);
            assert!(top >= column.top, "{top} < {}", column.top);
            beyond_unscaled |= column.base < kind.base - 300.0 && column.top > kind.base + 1_300.0;
        }
    }
    assert!(beyond_unscaled, "the fixture must exercise scale above one");
}
