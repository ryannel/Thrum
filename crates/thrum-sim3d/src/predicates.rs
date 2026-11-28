// Use the rounding residual when a geometric predicate loses its leading bits.
// All components, including the residuals, use the rules profile's Number.
use thrum_core::profile::Number;

use crate::{Vector3, n};

pub(crate) fn sum<const N: usize>(values: [Number; N]) -> Number {
    sum_terms(&values, &mut [Number::ZERO; N])
}

fn sum_terms(values: &[Number], partials: &mut [Number]) -> Number {
    let mut count = 0;
    for &value in values {
        if value == Number::ZERO {
            continue;
        }
        let mut high = value;
        let mut next = 0;
        for index in 0..count {
            let (rounded, residual) = two_sum(high, partials[index]);
            if residual != Number::ZERO {
                partials[next] = residual;
                next += 1;
            }
            high = rounded;
        }
        partials[next] = high;
        count = next + 1;
    }
    partials[..count]
        .iter()
        .fold(Number::ZERO, |total, part| total + *part)
}

pub(crate) fn two_sum(a: Number, b: Number) -> (Number, Number) {
    let rounded = a + b;
    let recovered_b = rounded - a;
    let residual = (a - (rounded - recovered_b)) + (b - recovered_b);
    (rounded, residual)
}

/// Dot product of a sum of vectors, preserving cancellations between terms.
pub(crate) fn dot_sum<const N: usize>(axis: Vector3, vectors: [Vector3; N]) -> Number {
    let mut terms = [[Number::ZERO; 6]; N];
    for (terms, vector) in terms.iter_mut().zip(vectors) {
        let (x, x_tail) = product(axis.x, vector.x);
        let (y, y_tail) = product(axis.y, vector.y);
        let (z, z_tail) = product(axis.z, vector.z);
        *terms = [x, y, z, x_tail, y_tail, z_tail];
    }
    sum_terms(
        terms.as_flattened(),
        [[Number::ZERO; 6]; N].as_flattened_mut(),
    )
}

/// Sign of squared distance minus the squared radius, keeping differences
/// too small to affect either rounded length. Inputs share a local frame.
pub(crate) fn squared_gap(first: Vector3, second: Vector3, mut radius: Number) -> Number {
    let mut offsets = [
        (first.x, second.x),
        (first.y, second.y),
        (first.z, second.z),
    ]
    .map(|(a, b)| two_sum(a, -b));
    let mut maximum = (first - second).max_component().max(radius);
    if maximum == Number::ZERO {
        return Number::ZERO;
    }
    // Scale before squaring tiny coordinates. Scale the terms directly so
    // an accumulated multiplier cannot overflow.
    let factor = n(1 << 16);
    while maximum < Number::ONE {
        for (head, tail) in &mut offsets {
            *head = *head * factor;
            *tail = *tail * factor;
        }
        radius = radius * factor;
        maximum = maximum * factor;
    }
    let mut terms = [Number::ZERO; 20];
    for (index, (head, tail)) in offsets.into_iter().enumerate() {
        terms[index * 6..index * 6 + 6].copy_from_slice(&square_terms(head, tail));
    }
    let (squared, remainder) = product(radius, radius);
    terms[18..].copy_from_slice(&[-squared, -remainder]);
    sum(terms)
}

fn square_terms(head: Number, tail: Number) -> [Number; 6] {
    let (squared, remainder) = product(head, head);
    let (mixed, mixed_tail) = product(head + head, tail);
    let (small, small_tail) = product(tail, tail);
    [squared, remainder, mixed, mixed_tail, small, small_tail]
}

/// Compare the distance to a cylinder's axis with its radius in the
/// unnormalised cross-section. Squaring after dividing by the edge length
/// can move a point exactly on a turned cylinder's wall off the surface.
pub(crate) fn cylinder_gap(offset: Vector3, edge: Vector3, radius: Number) -> Number {
    let mut terms = [Number::ZERO; 36];
    for (index, (a, b, c, d, component)) in [
        (offset.y, edge.z, offset.z, edge.y, edge.x),
        (offset.z, edge.x, offset.x, edge.z, edge.y),
        (offset.x, edge.y, offset.y, edge.x, edge.z),
    ]
    .into_iter()
    .enumerate()
    {
        let (left, left_tail) = product(a, b);
        let (right, right_tail) = product(c, d);
        let (head, rounding) = two_sum(left, -right);
        let tail = sum([rounding, left_tail, -right_tail]);
        let (edge_radius, edge_tail) = product(component, radius);
        terms[index * 12..index * 12 + 6].copy_from_slice(&square_terms(head, tail));
        terms[index * 12 + 6..index * 12 + 12]
            .copy_from_slice(&square_terms(edge_radius, edge_tail).map(|term| -term));
    }
    sum(terms)
}

fn split(value: Number) -> (Number, Number) {
    // Binary32 has 24 significand bits. Separate them without changing the value.
    let scaled = n(4097) * value;
    let high = scaled - (scaled - value);
    (high, value - high)
}

fn product(a: Number, b: Number) -> (Number, Number) {
    let rounded = a * b;
    if a == Number::ZERO || b == Number::ZERO {
        return (rounded, Number::ZERO);
    }
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let residual = al * bl - (((rounded - ah * bh) - al * bh) - ah * bl);
    (rounded, residual)
}

fn difference_of_products(a: Number, b: Number, c: Number, d: Number) -> Number {
    let left = a * b;
    let right = c * d;
    let delta = left - right;
    if left == Number::ZERO
        || right == Number::ZERO
        || delta.abs() > left.abs().max(right.abs()) / n(1 << 22)
    {
        return delta;
    }
    let (_, left_tail) = product(a, b);
    let (_, right_tail) = product(c, d);
    sum([left, -right, left_tail, -right_tail])
}

pub(crate) fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        difference_of_products(a.y, b.z, a.z, b.y),
        difference_of_products(a.z, b.x, a.x, b.z),
        difference_of_products(a.x, b.y, a.y, b.x),
    )
}

// Rescale a small direction by powers of two before multiplying. Direction
// and zero tests are invariant under this scaling, including parallel inputs.
pub(crate) fn conditioned(mut vector: Vector3) -> Vector3 {
    let mut magnitude = vector.max_component();
    let factor = n(1 << 16);
    while magnitude > Number::ZERO && magnitude < Number::ONE {
        vector = vector * factor;
        magnitude = magnitude * factor;
    }
    vector
}

pub(crate) fn determinant(a: Vector3, b: Vector3, c: Vector3) -> Number {
    let mut terms = [Number::ZERO; 24];
    for (index, (left, right, weight)) in [
        (a.y, b.z, c.x),
        (a.z, b.x, c.y),
        (a.x, b.y, c.z),
        (a.z, b.y, -c.x),
        (a.x, b.z, -c.y),
        (a.y, b.x, -c.z),
    ]
    .into_iter()
    .enumerate()
    {
        let (high, low) = product(left, right);
        let (head, tail) = product(high, weight);
        let (small, remainder) = product(low, weight);
        terms[index * 4..index * 4 + 4].copy_from_slice(&[head, tail, small, remainder]);
    }
    sum(terms)
}
