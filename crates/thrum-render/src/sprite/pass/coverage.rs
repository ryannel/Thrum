//! Camera-relative admission and projected coverage accounting.

use super::{GpuRow, SpriteRect, SpriteScene};
use crate::sprite::{PIXEL_FILTER_RADIUS, SHEET_GUTTER, SpriteSampling, SpriteView};

pub(super) fn coverage_at(
    scene: &SpriteScene,
    row: &GpuRow,
    frame: u32,
    view: SpriteView,
    extent: [u32; 2],
    alpha: f32,
    measure: bool,
) -> Option<f64> {
    let sheet = &scene.sheets[row.sheet as usize];
    let rect = sheet.sheet.frames[frame as usize];
    let project = |high: f32, low: f32, axis: usize| {
        (f64::from(high) - view.centre[axis] + f64::from(low)) * f64::from(view.pixels_per_unit)
    };
    let position = [0, 1].map(|axis| {
        let before = project(row.previous[axis], row.previous[axis + 2], axis);
        let after = project(row.current[axis], row.current[axis + 2], axis);
        before + (after - before) * f64::from(alpha)
    });
    let limit = f64::from(f32::MAX) / 8.0;
    if position
        .into_iter()
        .any(|value| !(-limit..=limit).contains(&value))
    {
        return None;
    }
    let [w, h] = dimensions(rect, row, view.pixels_per_unit, alpha);
    // The quad can extend four source texels on each side.
    let [sx, sy] = [w / f64::from(rect.size[0]), h / f64::from(rect.size[1])];
    let gutter = f64::from(SHEET_GUTTER);
    if (w.abs() + h.abs()) + 2.0 * gutter * (sx.abs() + sy.abs()) > limit {
        return None;
    }
    if !measure {
        return Some(0.0);
    }
    let angle = row.rotation + alpha * row.rotation_delta;
    let mut pivot = [
        position[0] + f64::from(extent[0]) * 0.5,
        -position[1] + f64::from(extent[1]) * 0.5,
    ];
    let margin = if sheet.tiles.contains(&frame) {
        0.0
    } else {
        let filter_margin = match sheet.sampling {
            SpriteSampling::Pixel => f64::from(PIXEL_FILTER_RADIUS),
            SpriteSampling::Smooth => 1.0,
        };
        sx.abs().min(sy.abs()).recip().clamp(filter_margin, gutter)
    };
    let (sin, cos) = f64::from(angle).sin_cos();
    pivot[0] -= margin * (sx * cos + sy * sin);
    pivot[1] -= margin * (-sx * sin + sy * cos);
    let w = w + 2.0 * margin * sx;
    let h = h + 2.0 * margin * sy;
    if angle == 0.0 {
        return Some(
            axis_aligned_pixels(pivot[0], w, extent[0])
                * axis_aligned_pixels(pivot[1], h, extent[1]),
        );
    }
    Some(clipped_quad_area(
        pivot,
        [w * cos, -w * sin],
        [h * sin, h * cos],
        extent,
    ))
}

fn axis_aligned_pixels(start: f64, length: f64, extent: u32) -> f64 {
    let end = start + length;
    let first = (start.min(end) - 0.5).ceil().clamp(0.0, f64::from(extent));
    let last = (start.max(end) - 0.5).ceil().clamp(0.0, f64::from(extent));
    last - first
}

fn dimensions(rect: SpriteRect, row: &GpuRow, pixels_per_unit: f32, alpha: f32) -> [f64; 2] {
    [0, 1].map(|axis| {
        let scale = row.scales[axis] + (row.scales[axis + 2] - row.scales[axis]) * alpha;
        f64::from(scale) * f64::from(rect.size[axis]) * f64::from(pixels_per_unit)
    })
}

fn clipped_quad_area(pivot: [f64; 2], right: [f64; 2], down: [f64; 2], extent: [u32; 2]) -> f64 {
    let add = |a: [f64; 2], b: [f64; 2]| [a[0] + b[0], a[1] + b[1]];
    let mut points = [[0.0; 2]; 8];
    points[..4].copy_from_slice(&[
        pivot,
        add(pivot, right),
        add(add(pivot, right), down),
        add(pivot, down),
    ]);
    let extent = extent.map(f64::from);
    if points[..4]
        .iter()
        .all(|point| (0..2).all(|axis| (0.0..=extent[axis]).contains(&point[axis])))
    {
        return (right[0] * down[1] - right[1] * down[0]).abs();
    }
    let mut count = 4;
    for (axis, edge, sense) in [
        (0, 0.0, 1.0),
        (1, 0.0, 1.0),
        (0, extent[0], -1.0),
        (1, extent[1], -1.0),
    ] {
        if count == 0 {
            return 0.0;
        }
        let input = points;
        let mut before = input[count - 1];
        let mut before_inside = (before[axis] - edge) * sense >= 0.0;
        let mut output = 0;
        for &after in &input[..count] {
            let after_inside = (after[axis] - edge) * sense >= 0.0;
            if before_inside != after_inside {
                let t = (edge - before[axis]) / (after[axis] - before[axis]);
                points[output] = [0, 1].map(|i| before[i] + (after[i] - before[i]) * t);
                output += 1;
            }
            if after_inside {
                points[output] = after;
                output += 1;
            }
            before = after;
            before_inside = after_inside;
        }
        count = output;
    }
    (0..count)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % count];
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        .abs()
        * 0.5
}
