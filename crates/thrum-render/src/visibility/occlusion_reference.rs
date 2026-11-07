//! CPU reference for two-phase occlusion (design, "Pyramid" and "Test"): the
//! reverse-Z min depth pyramid and the conservative rectangle test. The GPU
//! pyramid and late cull are checked against these in M2. Reverse-Z means min
//! is the farthest depth, and a box is hidden only when its nearest depth is
//! strictly farther than everything in its rectangle.

/// One single-channel pyramid level, row-major.
pub(super) struct Level {
    pub extent: [u32; 2],
    pub texels: Vec<f32>,
}

impl Level {
    fn at(&self, x: u32, y: u32) -> f32 {
        self.texels[(y * self.extent[0] + x) as usize]
    }
}

/// The largest power of two not above `n` (`n >= 1`).
pub(super) fn previous_pow2(n: u32) -> u32 {
    1 << n.ilog2()
}

/// Source range `[start, end)` covered by texel `index` of `to` texels over `from`.
fn footprint(index: u32, to: u32, from: u32) -> (u32, u32) {
    let start = u64::from(index) * u64::from(from) / u64::from(to);
    let end = (u64::from(index) + 1) * u64::from(from);
    let end = end.div_ceil(u64::from(to)).min(u64::from(from));
    // Both bounds are at most `from`, which is a u32.
    (
        u32::try_from(start).unwrap_or(from),
        u32::try_from(end).unwrap_or(from),
    )
}

/// Build the pyramid from a row-major depth target of `extent`. Mip 0 is
/// `previous_pow2` per axis and takes the min over each texel's whole source
/// rectangle; later mips take 2×2 mins, and odd edges take the extra row and
/// column.
pub(super) fn build(depth: &[f32], extent: [u32; 2]) -> Vec<Level> {
    assert_eq!(depth.len(), (extent[0] * extent[1]) as usize);
    let size = [previous_pow2(extent[0]), previous_pow2(extent[1])];
    let mut texels = Vec::with_capacity((size[0] * size[1]) as usize);
    for y in 0..size[1] {
        let (y0, y1) = footprint(y, size[1], extent[1]);
        for x in 0..size[0] {
            let (x0, x1) = footprint(x, size[0], extent[0]);
            let mut min = f32::INFINITY;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    min = min.min(depth[(sy * extent[0] + sx) as usize]);
                }
            }
            texels.push(min);
        }
    }
    let mut levels = vec![Level {
        extent: size,
        texels,
    }];
    while levels.last().is_some_and(|level| level.extent != [1, 1]) {
        let previous = levels.last().expect("non-empty");
        let [pw, ph] = previous.extent;
        let size = [(pw / 2).max(1), (ph / 2).max(1)];
        let mut texels = Vec::with_capacity((size[0] * size[1]) as usize);
        for y in 0..size[1] {
            let y1 = if y + 1 == size[1] { ph } else { 2 * y + 2 };
            for x in 0..size[0] {
                let x1 = if x + 1 == size[0] { pw } else { 2 * x + 2 };
                let mut min = f32::INFINITY;
                for sy in (2 * y).min(ph - 1)..y1 {
                    for sx in (2 * x).min(pw - 1)..x1 {
                        min = min.min(previous.at(sx, sy));
                    }
                }
                texels.push(min);
            }
        }
        levels.push(Level {
            extent: size,
            texels,
        });
    }
    levels
}

/// gamma16 = 16 u / (1 - 16 u), u = 2^-24: one path's clip error per unit
/// magnitude (design, "Depth tolerance").
pub(super) const CLIP_GAMMA: f32 = 9.536_752e-7;
/// Two divides at 2.5 ulp, four for the rasterizer's interpolation, one for
/// the comparison.
const DEPTH_ULPS: f32 = 10.0;

/// The rasterizer's vertex snap, in pixels: 8 sub-pixel bits, rounded.
const SNAP_PIXELS: f32 = 1.0 / 512.0;

fn depth_ulp(depth: f32) -> f32 {
    f32::from_bits(depth.abs().to_bits() + 1) - depth.abs()
}

/// How far a corner's uv on one axis may sit from where either path puts it,
/// beyond the quarter level-0 texel that the coverage rule already grants
/// (design, "Footprint"): `ndc` is its projected coordinate, `axis_magnitude`
/// that axis's clip term, `w_low` the corner's least possible w and `size` the
/// level-0 extent on that axis.
pub(super) fn footprint_widening(
    ndc: f32,
    w_low: f32,
    axis_magnitude: f32,
    w_magnitude: f32,
    size: f32,
) -> f32 {
    let clip = CLIP_GAMMA * (axis_magnitude + ndc.abs() * w_magnitude) / w_low;
    let rounding = 2.0 * depth_ulp(ndc) + 4.0 * depth_ulp(1.0);
    (clip + rounding + SNAP_PIXELS / size - 0.25 / size).max(0.0)
}

/// The derived depth slack for a box whose nearest corner depth is `nearest`
/// and least corner w is `w_min`, or `None` when the clip bound reaches w = 0
/// and the box stays visible (design, "Depth tolerance").
pub(super) fn slack(nearest: f32, w_min: f32, magnitude: [f32; 4]) -> Option<f32> {
    let w_low = w_min - 2.0 * CLIP_GAMMA * magnitude[3];
    (w_low > 0.0).then(|| {
        2.0 * CLIP_GAMMA * (magnitude[2] + nearest.abs() * magnitude[3]) / w_low
            + DEPTH_ULPS * depth_ulp(nearest)
    })
}

/// Whether a box whose eight clip-space corners are `corners` is hidden by the
/// pyramid. The caller has already kept it by frustum; a corner at or behind
/// the near plane keeps it visible. `magnitude` bounds the |clip x|, |clip y|,
/// |clip z| and |clip w| terms of every position the vertex shader or the
/// corners form.
pub(super) fn occluded(corners: &[[f32; 4]; 8], pyramid: &[Level], magnitude: [f32; 4]) -> bool {
    let size = pyramid[0].extent;
    let mut nearest = f32::NEG_INFINITY;
    let mut w_min = f32::INFINITY;
    let mut low = [u32::MAX; 2];
    let mut high = [0u32; 2];
    for &[x, y, z, w] in corners {
        // Reverse-Z: the near plane is z = w.
        if w <= 0.0 || z >= w {
            return false;
        }
        let corner_low = w - 2.0 * CLIP_GAMMA * magnitude[3];
        if corner_low <= 0.0 {
            return false;
        }
        nearest = nearest.max(z / w);
        w_min = w_min.min(w);
        let ndc = [x / w, y / w];
        let uv = [0.5 + 0.5 * ndc[0], 0.5 - 0.5 * ndc[1]];
        for axis in 0..2 {
            #[expect(
                clippy::cast_precision_loss,
                reason = "texture extents are far below 2^24"
            )]
            let extent = size[axis] as f32;
            let widening =
                footprint_widening(ndc[axis], corner_low, magnitude[axis], magnitude[3], extent);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped texel coordinate within a u32 extent"
            )]
            let texel = |uv: f32| (uv * extent).floor().clamp(0.0, extent - 1.0) as u32;
            low[axis] = low[axis].min(texel(uv[axis] - widening));
            high[axis] = high[axis].max(texel(uv[axis] + widening));
        }
    }
    let level = (0..pyramid.len())
        .find(|&level| (0..2).all(|axis| (high[axis] >> level) - (low[axis] >> level) <= 1))
        .expect("the 1×1 level always fits");
    let mip = &pyramid[level];
    let mut farthest = f32::INFINITY;
    let shift = u32::try_from(level).expect("few levels");
    for y in (low[1] >> shift)..=(high[1] >> shift).min(mip.extent[1] - 1) {
        for x in (low[0] >> shift)..=(high[0] >> shift).min(mip.extent[0] - 1) {
            farthest = farthest.min(mip.at(x, y));
        }
    }
    slack(nearest, w_min, magnitude).is_some_and(|slack| nearest + slack < farthest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Corners given exactly, as these tests construct them: no clip term.
    const EXACT: [f32; 4] = [0.0; 4];

    struct Lcg(u64);
    impl Lcg {
        fn unit(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[expect(clippy::cast_precision_loss, reason = "24 random bits fit f32 exactly")]
            let unit = (self.0 >> 40) as f32 / (1u64 << 24) as f32;
            unit
        }
        fn below(&mut self, n: u32) -> u32 {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "small test ranges"
            )]
            let value = (self.unit() * n as f32) as u32;
            value.min(n - 1)
        }
    }

    /// A screen-aligned box: NDC rectangle at one NDC depth, as clip corners with w = 1.
    fn corners(x: [f32; 2], y: [f32; 2], z: f32) -> [[f32; 4]; 8] {
        let mut out = [[0.0; 4]; 8];
        for (index, corner) in out.iter_mut().enumerate() {
            *corner = [x[index & 1], y[(index >> 1) & 1], z, 1.0];
        }
        out
    }

    /// Ground truth: drawn at its nearest depth, the box passes the Greater
    /// (or equal) test at some covered pixel.
    fn truly_visible(depth: &[f32], extent: [u32; 2], x: [f32; 2], y: [f32; 2], z: f32) -> bool {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "clamped pixel coordinate"
        )]
        let pixel = |uv: f32, n: u32, up: bool| {
            let p = uv * n as f32;
            let p = if up { p.ceil() } else { p.floor() };
            p.clamp(0.0, n as f32) as u32
        };
        let (x0, x1) = (
            pixel(0.5 + 0.5 * x[0], extent[0], false),
            pixel(0.5 + 0.5 * x[1], extent[0], true),
        );
        let (y0, y1) = (
            pixel(0.5 - 0.5 * y[1], extent[1], false),
            pixel(0.5 - 0.5 * y[0], extent[1], true),
        );
        (y0..y1.max(y0 + 1).min(extent[1])).any(|py| {
            (x0..x1.max(x0 + 1).min(extent[0])).any(|px| z >= depth[(py * extent[0] + px) as usize])
        })
    }

    #[test]
    fn previous_pow2_keeps_exact_powers() {
        assert_eq!(
            [1, 2, 3, 719, 720, 1024, 1280].map(previous_pow2),
            [1, 2, 2, 512, 512, 1024, 1024]
        );
    }

    #[test]
    fn every_texel_is_the_min_of_its_footprint() {
        for extent in [[1, 1], [3, 5], [7, 1], [1281, 719], [1280, 720]] {
            let mut random = Lcg(u64::from(extent[0]) * 31 + u64::from(extent[1]));
            let depth: Vec<f32> = (0..extent[0] * extent[1]).map(|_| random.unit()).collect();
            let pyramid = build(&depth, extent);
            assert_eq!(pyramid.last().expect("levels").extent, [1, 1]);
            let global = depth.iter().copied().fold(f32::INFINITY, f32::min);
            assert_eq!(
                pyramid.last().expect("levels").texels[0],
                global,
                "{extent:?}"
            );
            // A texel at any level is the min over its source footprint.
            for (level, mip) in pyramid.iter().enumerate() {
                let shift = u32::try_from(level).expect("few levels");
                for _ in 0..64 {
                    let (tx, ty) = (random.below(mip.extent[0]), random.below(mip.extent[1]));
                    let base = pyramid[0].extent;
                    let span = |t: u32, n: u32, b: u32| {
                        let start = t << shift;
                        let end = if t + 1 == n { b } else { (t + 1) << shift };
                        (start.min(b - 1), end)
                    };
                    let (bx0, bx1) = span(tx, mip.extent[0], base[0]);
                    let (by0, by1) = span(ty, mip.extent[1], base[1]);
                    let (sx0, _) = footprint(bx0, base[0], extent[0]);
                    let (_, sx1) = footprint(bx1 - 1, base[0], extent[0]);
                    let (sy0, _) = footprint(by0, base[1], extent[1]);
                    let (_, sy1) = footprint(by1 - 1, base[1], extent[1]);
                    let expected = (sy0..sy1)
                        .flat_map(|sy| (sx0..sx1).map(move |sx| (sx, sy)))
                        .map(|(sx, sy)| depth[(sy * extent[0] + sx) as usize])
                        .fold(f32::INFINITY, f32::min);
                    assert_eq!(
                        mip.at(tx, ty),
                        expected,
                        "{extent:?} level {level} ({tx},{ty})"
                    );
                }
            }
        }
    }

    #[test]
    fn never_hides_a_visible_box() {
        let mut hidden = 0;
        for extent in [[3, 5], [7, 1], [64, 48], [1281, 719], [1280, 720]] {
            let mut random = Lcg(u64::from(extent[0]) << 20 | u64::from(extent[1]));
            // Structured depth: a near wall over part of the view, far elsewhere, with noise.
            let wall = [random.below(extent[0]), random.below(extent[0])];
            let depth: Vec<f32> = (0..extent[0] * extent[1])
                .map(|index| {
                    let x = index % extent[0];
                    let near = (wall[0].min(wall[1])..=wall[0].max(wall[1])).contains(&x);
                    if near {
                        0.5 + 0.1 * random.unit()
                    } else {
                        0.01 * random.unit()
                    }
                })
                .collect();
            let pyramid = build(&depth, extent);
            for _ in 0..4_000 {
                // Rectangles may hang past the edges, so clamping is exercised.
                let a = [random.unit() * 2.4 - 1.2, random.unit() * 2.4 - 1.2];
                let b = [random.unit() * 0.6, random.unit() * 0.6];
                let x = [a[0], (a[0] + b[0]).max(a[0])];
                let y = [a[1], (a[1] + b[1]).max(a[1])];
                if x[1] < -1.0 || x[0] > 1.0 || y[1] < -1.0 || y[0] > 1.0 {
                    continue; // Outside the frustum: decided before occlusion.
                }
                let z = random.unit() * 0.7;
                let box_corners = corners(x, y, z);
                if occluded(&box_corners, &pyramid, EXACT) {
                    hidden += 1;
                    assert!(
                        !truly_visible(&depth, extent, x, y, z),
                        "{extent:?} hid a visible box x={x:?} y={y:?} z={z}"
                    );
                }
            }
        }
        assert!(hidden > 1_000, "the test must also hide work: {hidden}");
    }

    #[test]
    fn equality_and_the_near_plane_stay_visible() {
        let extent = [17, 9];
        let depth = vec![0.25; 17 * 9];
        let pyramid = build(&depth, extent);
        let x = [-0.3, 0.2];
        let y = [-0.4, 0.1];
        assert!(
            !occluded(&corners(x, y, 0.25), &pyramid, EXACT),
            "equal depth is visible"
        );
        // The spacing just below the power of two, where the nearest depth lies.
        let ulp = 0.25 - f32::from_bits(0.25_f32.to_bits() - 1);
        assert!(
            !occluded(&corners(x, y, 0.25 - 9.0 * ulp), &pyramid, EXACT),
            "within the depth ulps is visible"
        );
        assert!(
            occluded(&corners(x, y, 0.25 - 11.0 * ulp), &pyramid, EXACT),
            "past them is hidden"
        );
        // Positions of magnitude 10 under a z row of 0.1: the clip bound is
        // 2 gamma16 (0.1 + 0.25 * 10) / w, about 4.96e-6 at w = 1.
        let magnitude = [0.0, 0.0, 0.1, 10.0];
        assert!(
            !occluded(&corners(x, y, 0.25 - 4.5e-6), &pyramid, magnitude),
            "within the clip bound is visible"
        );
        assert!(
            occluded(&corners(x, y, 0.25 - 6.0e-6), &pyramid, magnitude),
            "past it is hidden"
        );
        assert!(
            occluded(&corners(x, y, 0.2), &pyramid, EXACT),
            "farther is hidden"
        );
        let mut crossing = corners(x, y, 0.2);
        crossing[3] = [0.0, 0.0, 1.0, 1.0];
        assert!(
            !occluded(&crossing, &pyramid, EXACT),
            "a corner on the near plane is visible"
        );
        crossing[3] = [0.0, 0.0, 0.5, -0.1];
        assert!(
            !occluded(&crossing, &pyramid, EXACT),
            "a corner behind the eye is visible"
        );
    }

    #[test]
    fn a_misaligned_rectangle_reads_its_whole_footprint() {
        // One near-depth hole straddling a level boundary must keep the box visible.
        let extent = [64, 64];
        let mut depth = vec![0.9; 64 * 64];
        depth[31 * 64 + 32] = 0.0;
        let pyramid = build(&depth, extent);
        let texel = |t: f32| t / 64.0 * 2.0 - 1.0;
        // Covers texels 31..=32 in x and 30..=32 in y.
        let x = [texel(31.5), texel(32.5)];
        let y = [-texel(32.5), -texel(30.5)];
        assert!(!occluded(
            &corners([x[0], x[1]], [y[0], y[1]], 0.5),
            &pyramid,
            EXACT
        ));
    }
}
