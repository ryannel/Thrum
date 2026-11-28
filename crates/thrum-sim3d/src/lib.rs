//! Collision pair tests in the portable rules profile.

mod hull;
mod pair;
mod predicates;
mod ray;
mod shape;
mod sweep;
mod vector;

pub use pair::{Penetration, depth, distance, overlap};
pub use ray::{RayHit, ray};
pub use shape::{DOMAIN_BOUND, QueryError, Shape};
pub use sweep::{
    CONTACT_TOLERANCE_DENOMINATOR, MAX_HALF_GAP_MOTION, SweepGap, SweepHit, sweep, sweep_two,
};
pub use vector::Vector3;

use thrum_core::profile::Number;

fn n(value: i32) -> Number {
    Number::from_i32(value)
}
