// Use the rounding residual when a geometric predicate loses its leading bits.
// All components, including the residuals, use the rules profile's Number.
use thrum_core::profile::Number;

use crate::{Vector3, n};

pub(crate) fn sum<const N: usize>(values: [Number; N]) -> Number {
    let mut partials = [Number::ZERO; N];
    let mut count = 0;
    for value in values {
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

fn two_sum(a: Number, b: Number) -> (Number, Number) {
    let rounded = a + b;
    let recovered_b = rounded - a;
    let residual = (a - (rounded - recovered_b)) + (b - recovered_b);
    (rounded, residual)
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
