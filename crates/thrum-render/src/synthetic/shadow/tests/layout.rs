use super::*;

fn view(eye: Vec3, target: Vec3, fov: f32, near: f32, far: f32) -> SyntheticView {
    SyntheticView::perspective(
        eye.to_array(),
        target.to_array(),
        Vec3::Y.to_array(),
        fov,
        1.5,
        near,
        far,
    )
    .unwrap()
}

#[test]
fn sun_cascade_layout_is_radial_snapped_and_has_upstream_depth() {
    let eye = Vec3::new(61.18, 35.91, -27.8);
    let a = view(eye, eye + Vec3::NEG_Z, 0.7, 0.1, 5000.0);
    let b = view(eye, eye + Vec3::new(5.0, 7.0, 3.0), 1.1, 0.8, 100.0);
    let moved = view(
        eye + Vec3::ONE,
        eye + Vec3::ONE + Vec3::NEG_Z,
        0.7,
        0.1,
        2000.0,
    );
    for degrees in [1.0_f32, 5.0, 35.0, 90.0] {
        let (y, z) = degrees.to_radians().sin_cos();
        let sun = Vec3::new(0.0, y, z);
        let upstream = eye + sun * 7000.0;
        let bounds = ShadowBounds::new(
            (upstream - Vec3::ONE).to_array(),
            (upstream + Vec3::ONE).to_array(),
        )
        .unwrap();
        let fit = |camera: &SyntheticView| {
            FitInputs::new(camera, (&[], None), sun)
                .unwrap()
                .fit(Some(bounds))
                .unwrap()
        };
        let original = fit(&a);
        let turned = fit(&b);
        let translated = fit(&moved);
        assert_eq!(
            original
                .matrices
                .map(|m| m.to_cols_array().map(f32::to_bits)),
            turned.matrices.map(|m| m.to_cols_array().map(f32::to_bits))
        );
        assert_eq!(original.samples.profile[0], SUN_CASCADE_COUNT as f32);
        assert_eq!(original.samples.profile[1], RECEIVER_REACH);
        for (index, &[radius, texel, _, _]) in original.samples.cascade_metrics.iter().enumerate() {
            let matrix = original.matrices[index];
            if index > 0 {
                assert_eq!(radius, 2.0 * original.samples.cascade_metrics[index - 1][0]);
                assert_eq!(texel, 2.0 * original.samples.cascade_metrics[index - 1][1]);
            }
            let half_width = matrix.row(0).truncate().length().recip();
            assert!(
                (half_width - radius - (CASCADE_GUARD_TEXELS + CASCADE_SNAP_TEXELS) * texel).abs()
                    < 5e-4
            );
            let drawing_offset = if radius <= SUN_OUTLINE_RADIUS {
                let [x, y] = SUN_OUTLINE_OFFSETS[0];
                Vec3::new(-x, y, 0.0)
            } else {
                Vec3::ZERO
            };
            let centre_in_texels =
                (matrix.project_point3(eye) * (SIZE as f32 * 0.5) - drawing_offset).abs();
            let movement =
                (translated.matrices[index].w_axis - matrix.w_axis) * (SIZE as f32 * 0.5);
            for axis in 0..2 {
                assert!(centre_in_texels[axis] <= CASCADE_SNAP_TEXELS + 0.001);
                assert!((movement[axis] - movement[axis].round()).abs() < 0.001);
                // Extremal projections of the whole world-space receiver ball.
                let bound = centre_in_texels[axis] + radius / texel;
                assert!(bound <= SIZE as f32 * 0.5 - CASCADE_GUARD_TEXELS + 0.001);
            }
            assert!((0.0..=1.0).contains(&matrix.project_point3(upstream).z));
        }
    }
}

#[test]
fn sun_cascade_layout_places_the_four_drawings_on_a_rotated_grid() {
    let camera = view(Vec3::ZERO, Vec3::NEG_Z, 0.7, 0.1, 100.0);
    let fit = FitInputs::new(&camera, (&[], None), Vec3::Y)
        .unwrap()
        .fit(None)
        .unwrap();
    assert_eq!(outline_count(SUN_CASCADE_COUNT, RECEIVER_REACH), 3);
    assert_eq!(outline_count(1, 128.0), 0);
    assert_eq!(outline_count(2, 16.0), 2);

    let expected = [
        [-0.125, -0.375],
        [0.375, -0.125],
        [-0.375, 0.125],
        [0.125, 0.375],
    ];
    for cascade in 0..3 {
        let mut drawings = [0; 4];
        drawings[0] = cascade;
        for (sample, layer) in drawings.iter_mut().enumerate().skip(1) {
            *layer = MAPS + 3 * cascade + sample - 1;
        }
        for (layer, [x, y]) in drawings.into_iter().zip(expected) {
            let draw = fit.caster_view(layer, MAPS);
            let matrix = Mat4::from_cols_array_2d(&draw.light_view_proj);
            let phase = matrix.w_axis * (SIZE as f32 / 2.0);
            assert_eq!([-phase.x, phase.y], [x, y]);
            assert_eq!(matrix.row(2), fit.matrices[cascade].row(2));
            assert_eq!(
                draw.minimum_cast_width,
                fit.caster_view(cascade, MAPS).minimum_cast_width
            );
        }
    }
}

#[test]
fn sun_cascade_layout_profiles_and_bias() {
    let camera = view(Vec3::ZERO, Vec3::NEG_Z, 0.7, 0.1, 100.0);
    for count in [0, 1, 3, SUN_CASCADE_COUNT] {
        let selected = camera.with_sun_shadow_profile(count, 128.0).unwrap();
        let fit = FitInputs::new(&selected, (&[], None), Vec3::Y)
            .unwrap()
            .fit(None)
            .unwrap();
        assert_eq!(fit.samples.profile[..2], [count as f32, 128.0]);
        for (index, metrics) in fit.samples.cascade_metrics[..count as usize]
            .iter()
            .enumerate()
        {
            let [radius, texel, inverse_depth, bias] = *metrics;
            assert_eq!(
                radius,
                128.0 / (1_u32 << (count as usize - index - 1)) as f32
            );
            assert!(
                (bias - (texel * inverse_depth * 0.25).max(MINIMUM_DEPTH_BIAS)).abs()
                    <= bias * 2.0 * f32::EPSILON
            );
            // The only fixed precision floor is in normalized depth, not metres.
            assert_eq!(
                fit.caster_view(index, MAPS).minimum_cast_width[0],
                texel * CAST_WIDTH_TEXELS
            );
        }
        assert!(
            fit.samples.cascade_metrics[count as usize..]
                .iter()
                .flatten()
                .all(|&value| value.to_bits() == 0)
        );
        assert_eq!(
            fit.caster_view(HEIGHT_LAYER, MAPS).minimum_cast_width[0],
            0.0
        );
    }
    for reach in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert!(camera.with_sun_shadow_profile(1, reach).is_err());
    }
    assert!(
        camera
            .with_sun_shadow_profile(SUN_CASCADE_COUNT + 1, 128.0)
            .is_err()
    );
    const { assert!(CAST_WIDTH_TEXELS >= std::f32::consts::SQRT_2) };
}

#[derive(Default)]
pub(super) struct Geometry {
    vertices: Vec<crate::synthetic::SyntheticVertex>,
    indices: Vec<u32>,
}

impl Geometry {
    fn triangle(&mut self, positions: [Vec3; 3]) {
        for position in positions {
            self.indices
                .push(u32::try_from(self.vertices.len()).unwrap());
            self.vertices.push(crate::synthetic::SyntheticVertex {
                position: position.to_array(),
                normal: [0.0, 1.0, 0.0],
                uv: [0.0; 2],
                tangent: [1.0, 0.0, 0.0, 1.0],
            });
        }
    }

    fn quad(&mut self, [a, b, c, d]: [Vec3; 4]) {
        self.triangle([a, b, c]);
        self.triangle([a, c, d]);
    }

    pub(super) fn box_at(&mut self, centre: Vec3, size: Vec3) {
        for axis in 0..3 {
            let u = Vec3::AXES[(axis + 1) % 3];
            let v = Vec3::AXES[(axis + 2) % 3];
            for side in [-1.0, 1.0] {
                let n = side * Vec3::AXES[axis];
                let mut face = [n - u - v, n + u - v, n + u + v, n - u + v]
                    .map(|point| centre + 0.5 * size * point);
                if side < 0.0 {
                    face.swap(1, 3);
                }
                self.quad(face);
            }
        }
    }

    fn radial(&mut self, sphere: bool) {
        const SIDES: usize = 24;
        const RINGS: usize = 12;
        let point = |ring: usize, side: usize| {
            let longitude = (side % SIDES) as f32 * std::f32::consts::TAU / SIDES as f32;
            let (z, x) = longitude.sin_cos();
            if sphere {
                if ring == 0 {
                    return Vec3::NEG_Y;
                }
                if ring == RINGS {
                    return Vec3::Y;
                }
                let latitude = (ring as f32 / RINGS as f32 - 0.5) * std::f32::consts::PI;
                let (y, radius) = latitude.sin_cos();
                Vec3::new(radius * x, y, radius * z)
            } else {
                Vec3::new(x, if ring == 0 { -2.0 } else { 2.0 }, z)
            }
        };
        let rings = if sphere { RINGS } else { 1 };
        for ring in 0..rings {
            for side in 0..SIDES {
                self.quad([
                    point(ring, side),
                    point(ring, side + 1),
                    point(ring + 1, side + 1),
                    point(ring + 1, side),
                ]);
                if !sphere {
                    self.triangle([Vec3::NEG_Y * 2.0, point(0, side), point(0, side + 1)]);
                    self.triangle([Vec3::Y * 2.0, point(1, side + 1), point(1, side)]);
                }
            }
        }
    }

    pub(super) fn mesh(self) -> crate::synthetic::SyntheticMesh {
        crate::synthetic::SyntheticMesh {
            vertices: self.vertices,
            indices: self.indices,
        }
    }

    fn width(&self) -> f32 {
        let bounds = self.vertices.iter().fold(
            [Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)],
            |[lo, hi], vertex| {
                let position = Vec3::from_array(vertex.position);
                [lo.min(position), hi.max(position)]
            },
        );
        cast_width::measure(&self.vertices, &self.indices, bounds.map(|p| p.to_array()))
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "The test geometry is bounded to a few grid cells"
    )]
    fn covers_grid_centre(&self, rotation: Mat4, scale: f32, offset: glam::Vec2) -> bool {
        self.indices.as_chunks::<3>().0.iter().any(|triangle| {
            let points = triangle.map(|index| {
                let position = Vec3::from_array(self.vertices[index as usize].position);
                rotation.transform_vector3(position * scale).truncate() + offset
            });
            let min = points[0]
                .min(points[1])
                .min(points[2])
                .ceil()
                .to_array()
                .map(|v| v as i32);
            let max = points[0]
                .max(points[1])
                .max(points[2])
                .floor()
                .to_array()
                .map(|v| v as i32);
            (min[0]..=max[0]).any(|x| {
                (min[1]..=max[1]).any(|y| {
                    let p = glam::Vec2::new(x as f32, y as f32);
                    let [a, b, c] = points;
                    let signs = [
                        (b - a).perp_dot(p - a),
                        (c - b).perp_dot(p - b),
                        (a - c).perp_dot(p - c),
                    ];
                    signs.iter().all(|&sign| sign >= 0.0) || signs.iter().all(|&sign| sign <= 0.0)
                })
            })
        })
    }
}

#[test]
fn sun_cascade_layout_cast_width_uses_triangles_and_covers_texel_centres() {
    let mut box_mesh = Geometry::default();
    box_mesh.box_at(Vec3::ZERO, Vec3::new(4.0, 2.0, 3.0));
    let mut post = Geometry::default();
    post.box_at(Vec3::ZERO, Vec3::new(2.0, 10.0, 2.0));
    for side in [-1.0, 1.0] {
        post.box_at(Vec3::new(2.0 * side, 2.5, 0.0), Vec3::new(3.0, 1.0, 1.0));
    }
    let mut sphere = Geometry::default();
    sphere.radial(true);
    let mut cylinder = Geometry::default();
    cylinder.radial(false);
    let mut sheet = Geometry::default();
    sheet.quad([
        Vec3::new(-2.0, 0.0, -1.0),
        Vec3::new(-2.0, 0.0, 1.0),
        Vec3::new(2.0, 0.0, 1.0),
        Vec3::new(2.0, 0.0, -1.0),
    ]);
    for (name, geometry) in [
        ("box", &box_mesh),
        ("sphere", &sphere),
        ("cylinder", &cylinder),
        ("post with arms", &post),
        ("sheet", &sheet),
    ] {
        let width = geometry.width();
        assert!((1.5..=2.0).contains(&width), "{name} cast width {width}");
    }
    for geometry in [&box_mesh, &sphere, &post] {
        let scale = CAST_WIDTH_TEXELS / geometry.width();
        for elevation in [10.0_f32, 35.0, 60.0] {
            let (y, z) = elevation.to_radians().sin_cos();
            let rotation = light_rotation(Vec3::new(0.0, y, z));
            for x in 0..12 {
                for y in 0..12 {
                    let offset = glam::Vec2::new(x as f32, y as f32) / 12.0;
                    assert!(
                        geometry.covers_grid_centre(rotation, scale, offset),
                        "elevation {elevation}, grid offset {offset}"
                    );
                }
            }
        }
    }
}

#[test]
fn sun_cascade_layout_membership_uses_smallest_axis_instead_of_bounds() {
    let camera = view(Vec3::ZERO, Vec3::NEG_Z, 0.7, 0.1, 5000.0);
    let mut instance = InstanceRecord::identity(0, 0);
    instance.current = Mat4::from_scale_rotation_translation(
        Vec3::new(1.7, 0.926, 2.4),
        glam::Quat::from_rotation_y(0.7),
        Vec3::ZERO,
    )
    .to_cols_array_2d();
    let width = CasterBox::world_cast_width(0.324, &instance);
    assert!((width - 0.3).abs() < 0.001);
    for degrees in [1.0_f32, 5.0, 16.0, 35.0, 90.0] {
        let (y, z) = degrees.to_radians().sin_cos();
        let sun = Vec3::new(0.0, y, z);
        let fit = FitInputs::new(&camera, (&[], None), sun)
            .unwrap()
            .fit(None)
            .unwrap();
        let layer_at = |metres: f32| {
            fit.samples
                .cascade_metrics
                .iter()
                .position(|metrics| (metrics[1] - metres).abs() < 0.035)
                .unwrap()
        };
        // Review's cactus bounds (1.86 × 4.06 texels) would admit it at 0.5 m.
        let rejected = layer_at(0.5);
        let texel = fit.samples.cascade_metrics[rejected][1];
        assert!((width / texel - 0.6).abs() < 0.04);
        for layer in rejected..fit.count as usize {
            assert!(width < fit.caster_view(layer, MAPS).minimum_cast_width[0]);
        }
        let accepted = layer_at(0.125);
        assert!(width >= fit.caster_view(accepted, MAPS).minimum_cast_width[0]);
    }
}

#[test]
fn sun_cascade_layout_packs_height_support_and_solar_lower_limb() {
    let camera = view(Vec3::ZERO, Vec3::NEG_Z, 0.7, 0.1, 100.0);
    let direction = Vec3::ONE.normalize();
    let bounds = ShadowBounds::new([-100.0; 3], [100.0, 2000.0, 100.0]).unwrap();
    let fit = FitInputs::new(&camera, (&[], None), direction)
        .unwrap()
        .fit(Some(bounds))
        .unwrap();
    let [height, lower, _, _] = fit.samples.height_support;
    assert_eq!(height, 2000.0);
    assert!(lower > 0.0 && lower < direction.y);
    let tangent = direction.cross(Vec3::Y).normalize();
    let bitangent = direction.cross(tangent);
    for i in 0..32 {
        let phase = i as f32 * std::f32::consts::TAU / 32.0;
        let ray = direction * SUN_ANGULAR_RADIUS.cos()
            + (tangent * phase.cos() + bitangent * phase.sin()) * SUN_ANGULAR_RADIUS.sin();
        assert!(ray.y >= lower - f32::EPSILON);
    }
}
