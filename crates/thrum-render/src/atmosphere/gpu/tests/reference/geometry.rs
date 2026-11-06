use std::ops::{Add, Mul, Sub};

pub(super) const RADIUS: f64 = 6_371_000.0;
pub(super) const HEIGHT: f64 = 100_000.0;

#[derive(Clone, Copy, Debug)]
pub(super) struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Vec3 {
    pub(super) fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub(super) fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    pub(super) fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub(super) fn unit(self) -> Self {
        self * (1.0 / self.length())
    }
    pub(super) fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
}
impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}
impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

fn roots(a: f64, b: f64, c: f64) -> Option<[f64; 2]> {
    // a*t^2 + 2*b*t + c. The q form avoids cancellation of the near root.
    if a.abs() < 1e-15 {
        return (b.abs() > 1e-15).then(|| [-c / (2.0 * b); 2]);
    }
    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let q = -b - discriminant.sqrt().copysign(b);
    if q == 0.0 {
        return Some([0.0; 2]);
    }
    let mut result = [q / a, c / q];
    result.sort_by(f64::total_cmp);
    Some(result)
}

fn sphere(origin: Vec3, direction: Vec3, radius: f64) -> Option<[f64; 2]> {
    roots(
        direction.dot(direction),
        origin.dot(direction),
        origin.dot(origin) - radius * radius,
    )
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Boundary {
    pub distance: f64,
    pub ground: bool,
}

pub(super) fn boundary(origin: Vec3, direction: Vec3) -> Boundary {
    let top = sphere(origin, direction, RADIUS + HEIGHT).map_or(0.0, |t| t[1].max(0.0));
    let height = origin.length() - RADIUS;
    let inward = origin.dot(direction) < 0.0;
    let ground = if height.abs() < 1e-6 {
        inward.then_some(0.0)
    } else if inward {
        sphere(origin, direction, RADIUS).and_then(|t| (t[0] >= 0.0).then_some(t[0]))
    } else {
        None
    };
    ground.filter(|&distance| distance <= top).map_or(
        Boundary {
            distance: top,
            ground: false,
        },
        |distance| Boundary {
            distance,
            ground: true,
        },
    )
}

pub(super) fn splits(origin: Vec3, direction: Vec3, length: f64, sun: Option<Vec3>) -> Vec<f64> {
    let mut points = vec![0.0, length];
    let mut add = |value: f64| {
        if value > 0.0 && value < length {
            points.push(value);
        }
    };
    add(-origin.dot(direction) / direction.dot(direction));
    for height in [10_000.0, 25_000.0, 40_000.0] {
        if let Some(values) = sphere(origin, direction, RADIUS + height) {
            for value in values {
                add(value);
            }
        }
    }
    if let Some(sun) = sun {
        let os = origin.dot(sun);
        let ds = direction.dot(sun);
        // Shadow cylinder and its terminator plane. Extra roots outside the
        // actual shadow half-cylinder are harmless quadrature subdivisions.
        if let Some(values) = roots(
            direction.dot(direction) - ds * ds,
            origin.dot(direction) - os * ds,
            origin.dot(origin) - os * os - RADIUS * RADIUS,
        ) {
            for value in values {
                add(value);
            }
        }
        if ds.abs() > 1e-15 {
            add(-os / ds);
        }
    }
    points.sort_by(f64::total_cmp);
    points.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    points
}
