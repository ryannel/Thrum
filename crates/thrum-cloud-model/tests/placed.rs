//! Asymmetric voxel footprints bound volume, column and every tested blend.
use crate::support;
use support::{close, field, formed, shape};
use thrum_cloud_model::Density;

#[test]
fn footprint_is_the_pointwise_voxel_integral() {
    let shape = shape();
    let state = formed(&shape);
    let field = field(7, 0.3);
    for x in -12..=12 {
        for z in -10..=10 {
            let point = [f64::from(x) * 5.0, 700.0, f64::from(z) * 5.0];
            let c = field.query(point, &state);
            // Independent slab quadrature, including horizontal interpolation and hollow.
            let voxels: f64 = (0..6)
                .map(|y| {
                    shape.density([point[0], f64::from(y) * 20.0 + 10.0, point[2]]) * 20.0 / 20.0
                })
                .sum();
            close(c.optical_depth, voxels, 1e-12); // fixture extinction = 0.05 m^-1
            for weight in [0.0, 0.25, 0.5, 0.75, 1.0] {
                for fidelity in [Density::Uneroded, Density::Full, Density::Coarse] {
                    let tau: f64 = (0..480)
                        .map(|i| {
                            field.density(
                                [point[0], 700.0 + (f64::from(i) + 0.5) * 0.25, point[2]],
                                &state,
                                fidelity,
                                |_| Some(&shape),
                                weight,
                            ) * 0.25
                        })
                        .sum();
                    // Midpoint bound of the independently integrated smooth profile.
                    assert!(
                        tau <= c.optical_depth * (1.0 + 2e-5) + 1e-9,
                        "{tau} > {}",
                        c.optical_depth
                    );
                    if weight > 0.99 && matches!(fidelity, Density::Uneroded) {
                        close(tau, c.optical_depth, 1e-10);
                    }
                    if c.optical_depth == 0.0 {
                        close(tau, 0.0, 0.0);
                    }
                }
            }
        }
    }
}

#[test]
fn hashed_cells_use_the_same_camera_independent_footprints() {
    use std::sync::Arc;
    use thrum_cloud_model::{Fidelity, HashedShapes, PlacedLibrary, PlacedSlab, Shape, Weather};
    let cube = Shape::new([4; 3], 60.0, vec![0.75; 64]).expect("cube");
    let footprint = Arc::new(cube.footprint().clone());
    let library = Arc::new(
        PlacedLibrary::new(
            vec![PlacedSlab {
                extent: 240.0,
                floor: 0.0,
                heading: 0.0,
                footprint,
            }],
            Vec::new(),
        )
        .expect("library"),
    );
    let mut authored = support::state(1.0, 0.0);
    authored
        .set_hashed_library(
            library,
            HashedShapes {
                block_sites: 3,
                shares: vec![1.0; 2],
            },
        )
        .expect("hashed weather");
    let mut procedural = support::state(1.0, 0.0);
    authored.step(1, 600.0).expect("formed sky cells");
    procedural.step(1, 600.0).expect("matched weather");
    let model = field(7, 0.0);
    let weather = Weather {
        field: &model,
        surface: None,
        ground: None,
    };
    let block_width = model.placed_block_width(&authored).expect("blocks");
    let mut count = 0;
    for z in -3..=3 {
        for x in -3..=3 {
            let Some((shape, _)) = model.hashed_shape([x, z], &authored) else {
                continue;
            };
            let radius = shape.footprint.radius() * shape.scale;
            for (axis, block) in [x, z].into_iter().enumerate() {
                let first = f64::from(i32::try_from(block).expect("test block")) * block_width;
                assert!(shape.centre[axis] - radius > first);
                assert!(shape.centre[axis] + radius < first + block_width);
            }
            let point = [shape.centre[0], 0.0, shape.centre[1]];
            let added = model.query(point, &authored).optical_depth
                - model.query(point, &procedural).optical_depth;
            close(added, 180.0 * shape.envelope * shape.scale * 0.05, 1e-10);
            let point = [point[0], 700.0 + 120.0 * shape.scale, point[2]];
            let estimated = weather.extinction(point, &authored, |_| None);
            assert_eq!(estimated.fidelity, Fidelity::Footprint);
            assert!(estimated.value > weather.extinction(point, &procedural, |_| None).value);
            assert_eq!(
                weather
                    .extinction(point, &authored, |_| Some(&cube))
                    .fidelity,
                Fidelity::Exact
            );
            let path = weather
                .optical_depth(
                    [point[0] - 5.0, point[1], point[2]],
                    [point[0] + 5.0, point[1], point[2]],
                    &authored,
                    8,
                    Density::Coarse,
                    |_| None,
                )
                .expect("sight line");
            assert_eq!(path.fidelity, Fidelity::Footprint);
            count += 1;
        }
    }
    assert!(count >= 5);
}

#[test]
fn library_condensate_fades_without_crossing_an_erosion_cutoff() {
    use std::sync::Arc;
    use thrum_cloud_model::{PlacedLibrary, PlacedSlab, Shape, ShapeSource};
    let shape = Shape::new([4; 3], 20.0, vec![0.75; 64]).expect("cube");
    let slab = PlacedSlab {
        extent: 80.0,
        floor: 0.0,
        heading: 0.0,
        footprint: Arc::new(shape.footprint().clone()),
    };
    let library = Arc::new(PlacedLibrary::new(vec![slab], Vec::new()).expect("library"));
    let field = field(7, 0.61);
    let mut state = support::state(0.0, 0.0);
    state.set_library(library).expect("attach library");
    state
        .add_from_library(
            support::directed(&shape, 1, [0.0; 2]),
            ShapeSource::Library(0),
        )
        .expect("admit cube");
    state.step(1, 10.0).expect("form");
    let read = |state: &_, weight, fidelity| {
        field.density([0.0, 740.0, 0.0], state, fidelity, |_| Some(&shape), weight)
    };
    let mut fading = state.clone();
    fading.direct(1, 0.125, 0.0).expect("fade");
    fading.step(2, 10.0).expect("fade settled");
    state.step(2, 10.0).expect("same weather time");
    for weight in [0.0, 0.5, 1.0] {
        for fidelity in [Density::Coarse, Density::Full] {
            let full = read(&state, weight, fidelity);
            assert!(full > 0.0);
            close(read(&fading, weight, fidelity), 0.125 * full, 1e-12);
        }
    }
}

#[test]
fn authored_floor_places_voxels_in_world_space() {
    use std::sync::Arc;
    use thrum_cloud_model::{
        Fidelity, Footprint, PlacedLibrary, PlacedSlab, Shape, ShapeSource, Weather,
    };
    let floor = -85.0;
    let mut voxels = vec![0.0; 64];
    for z in 0..4 {
        for x in 0..4 {
            voxels[z * 16 + x] = 0.625;
        }
    }
    let shape = Shape::new([4; 3], 20.0, voxels).expect("asymmetric cube");
    let footprint = Footprint::from_columns(
        [4; 2],
        20.0,
        shape
            .footprint()
            .columns()
            .iter()
            .map(|&[tau, low, high]| [tau, low + floor, high + floor])
            .collect(),
    )
    .expect("condensation-relative columns");
    let library = PlacedLibrary::new(
        vec![PlacedSlab {
            extent: 80.0,
            floor,
            heading: 0.0,
            footprint: Arc::new(footprint),
        }],
        Vec::new(),
    )
    .expect("offset slab");
    let mut state = support::state(0.0, 0.0);
    state.set_library(Arc::new(library)).expect("library");
    let mut placed = support::directed(&shape, 2, [450.0, -300.0]);
    placed.scale = 1.5;
    state
        .add_from_library(placed, ShapeSource::Library(0))
        .expect("admitted slab");
    state.step(1, 10.0).expect("formed cloud");
    let field = field(7, 0.0);
    let weather = Weather {
        field: &field,
        surface: None,
        ground: None,
    };
    let y = 700.0 + 1.5 * (floor + 10.0);
    let expected = 0.625 * 0.05;
    let volume = |id| (id == 2).then_some(&shape);
    let inside = weather.extinction([450.0, y, -300.0], &state, volume);
    assert_eq!(inside.fidelity, Fidelity::Exact);
    close(inside.value, expected, 1e-12);
    let ray = weather
        .optical_depth(
            [435.0, y, -300.0],
            [465.0, y, -300.0],
            &state,
            30,
            Density::Coarse,
            volume,
        )
        .expect("occupied ray");
    assert_eq!(ray.fidelity, Fidelity::Exact);
    close(ray.value, 30.0 * expected, 1e-12);
    let clear = weather.extinction([450.0, y + 30.0, -300.0], &state, volume);
    close(clear.value, 0.0, 0.0);
}
