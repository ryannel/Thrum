//! Reusable derived circle grid. Query order is cell row, column, then slot.
use thrum_core::profile::{Number, StateEncode, Vector};
mod shape;
pub use shape::Shape;

/// An indexed circle with a game-selected logical slot.
#[derive(Clone, Copy, Debug)]
pub struct Circle {
    /// Logical slot, ascending in build input.
    pub slot: u32,
    /// Centre.
    pub centre: Vector,
    /// Nonnegative radius.
    pub radius: Number,
}

/// Invalid spatial data, refused before replacing the index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryError {
    /// Cell size is nonfinite or below 1/16.
    CellSize,
    /// A body is outside the query domain or out of slot order.
    Body(u32),
    /// A query is outside the query domain.
    Query,
}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for QueryError {}

#[derive(Clone, Copy, Default)]
struct Cell {
    key: Option<(i32, i32)>,
    start: usize,
    count: usize,
    written: usize,
}

/// A warmed build reuses the hash table, sorted rows and oversize list.
pub struct CircleIndex {
    size: Number,
    cells: Vec<Cell>,
    sorted: Vec<Circle>,
    oversize: Vec<Circle>,
    ordered_cells: Vec<(u64, usize)>,
    ordering_scratch: Vec<(u64, usize)>,
    maximum_radius: Number,
}

fn valid(value: Number) -> bool {
    value.is_finite() && value.abs() <= Number::from_i32(16_777_216)
}
fn valid_circle(circle: &Circle) -> bool {
    valid(circle.centre.x)
        && valid(circle.centre.y)
        && valid(circle.radius)
        && circle.radius >= Number::ZERO
}

impl CircleIndex {
    /// Create an empty grid with the rule set's fixed cell size.
    ///
    /// # Errors
    /// Invalid cell size.
    pub fn new(size: Number) -> Result<Self, QueryError> {
        if !size.is_finite() || size < Number::ONE / Number::from_i32(16) {
            return Err(QueryError::CellSize);
        }
        Ok(Self {
            size,
            cells: vec![Cell::default()],
            sorted: Vec::new(),
            oversize: Vec::new(),
            ordered_cells: Vec::new(),
            ordering_scratch: Vec::new(),
            maximum_radius: Number::ZERO,
        })
    }

    fn coordinates(&self, centre: Vector) -> (i32, i32) {
        (
            (centre.x / self.size).floor().to_i32(),
            (centre.y / self.size).floor().to_i32(),
        )
    }

    fn bucket(&self, key: (i32, i32)) -> usize {
        let x = u32::from_le_bytes(key.0.to_le_bytes());
        let y = u32::from_le_bytes(key.1.to_le_bytes());
        let hash = u64::from(x).wrapping_mul(0x9e37_79b9) ^ u64::from(y).wrapping_mul(0x85eb_ca6b);
        let mask = self.cells.len() - 1;
        let mut bucket = usize::try_from(hash & u64::try_from(mask).expect("mask fits"))
            .expect("masked hash fits");
        while self.cells[bucket].key.is_some_and(|found| found != key) {
            bucket = (bucket + 1) & mask;
        }
        bucket
    }

    /// Replace the grid from ascending logical slots. Validation precedes mutation.
    ///
    /// # Errors
    /// An invalid circle or nonascending slot.
    pub fn build(&mut self, circles: &[Circle]) -> Result<(), QueryError> {
        for (index, circle) in circles.iter().enumerate() {
            if !valid_circle(circle) || (index > 0 && circles[index - 1].slot >= circle.slot) {
                return Err(QueryError::Body(circle.slot));
            }
        }
        self.cells.resize(
            (circles.len() * 2).max(1).next_power_of_two(),
            Cell::default(),
        );
        self.cells.fill(Cell::default());
        self.oversize.clear();
        self.ordered_cells.clear();
        self.maximum_radius = Number::ZERO;
        for &circle in circles {
            if circle.radius > self.size {
                self.oversize.push(circle);
                continue;
            }
            self.maximum_radius = self.maximum_radius.max(circle.radius);
            let key = self.coordinates(circle.centre);
            let bucket = self.bucket(key);
            let cell = &mut self.cells[bucket];
            if cell.key.is_none() {
                cell.key = Some(key);
                let x = u32::from_le_bytes(key.0.to_le_bytes()) ^ 0x8000_0000;
                let y = u32::from_le_bytes(key.1.to_le_bytes()) ^ 0x8000_0000;
                self.ordered_cells
                    .push(((u64::from(y) << 32) | u64::from(x), bucket));
            }
            cell.count += 1;
        }
        let mut start = 0;
        for cell in &mut self.cells {
            cell.start = start;
            start += cell.count;
        }
        self.sorted.resize_with(start, || Circle {
            slot: 0,
            centre: Vector::default(),
            radius: Number::ZERO,
        });
        for &circle in circles {
            if circle.radius > self.size {
                continue;
            }
            let bucket = self.bucket(self.coordinates(circle.centre));
            let cell = &mut self.cells[bucket];
            self.sorted[cell.start + cell.written] = circle;
            cell.written += 1;
        }
        self.ordering_scratch
            .resize(self.ordered_cells.len(), (0, 0));
        // Wide queries traverse cells in row and column order. Sort each
        // occupied cell once. The scatter above already preserves slot order.
        for shift in (0..64).step_by(8) {
            let mut counts = [0; 256];
            for &(key, _) in &self.ordered_cells {
                counts[((key >> shift) & 255) as usize] += 1;
            }
            let mut start = 0;
            for count in &mut counts {
                let next = start + *count;
                *count = start;
                start = next;
            }
            for &entry in &self.ordered_cells {
                let bucket = ((entry.0 >> shift) & 255) as usize;
                self.ordering_scratch[counts[bucket]] = entry;
                counts[bucket] += 1;
            }
            std::mem::swap(&mut self.ordered_cells, &mut self.ordering_scratch);
        }
        Ok(())
    }

    /// Visit every touching circle in oversize then cell row, column, slot order.
    ///
    /// # Panics
    /// If a caller callback panics or an index exceeds the addressable body count.
    ///
    /// # Errors
    /// Query coordinates or radius outside the domain.
    pub fn overlap(
        &self,
        centre: Vector,
        radius: Number,
        visit: impl FnMut(&Circle),
    ) -> Result<(), QueryError> {
        self.query_matching(
            Shape::Circle { centre, radius },
            |circle| shape::circle_intersects(centre, radius, circle),
            visit,
        )
    }

    /// Visit overlaps in oversize-slot then cell-row, column, slot order.
    /// Queries are read-only, allocation-free and safe to share across rules.
    ///
    /// # Errors
    /// A shape coordinate or radius is outside the domain, or box bounds are reversed.
    ///
    /// # Panics
    /// If the caller's visitor panics or the body count exceeds i64.
    pub fn query(&self, shape: Shape, visit: impl FnMut(&Circle)) -> Result<(), QueryError> {
        if let Shape::Circle { centre, radius } = shape {
            return self.overlap(centre, radius, visit);
        }
        self.query_matching(shape, |circle| shape.intersects(circle), visit)
    }

    fn query_matching(
        &self,
        shape: Shape,
        accepts: impl Fn(&Circle) -> bool,
        mut visit: impl FnMut(&Circle),
    ) -> Result<(), QueryError> {
        if !shape.valid() {
            return Err(QueryError::Query);
        }
        for circle in &self.oversize {
            if accepts(circle) {
                visit(circle);
            }
        }
        let (minimum, maximum) = shape.bounds();
        // Outward slack covers binary32 subtraction/projection/bound rounding,
        // including tiny shapes that straddle zero. It changes candidate work,
        // never the predicate or canonical order; wide queries still scan bodies.
        let magnitude = minimum
            .x
            .abs()
            .max(minimum.y.abs())
            .max(maximum.x.abs())
            .max(maximum.y.abs())
            + self.maximum_radius;
        let slack = magnitude / Number::from_i32(1 << 20)
            + Number::ONE / Number::from_i32(1 << 30) / Number::from_i32(1 << 30);
        let low = self.coordinates(Vector {
            x: minimum.x - self.maximum_radius - slack,
            y: minimum.y - self.maximum_radius - slack,
        });
        let high = self.coordinates(Vector {
            x: maximum.x + self.maximum_radius + slack,
            y: maximum.y + self.maximum_radius + slack,
        });
        let area = i64::from(high.0 - low.0 + 1) * i64::from(high.1 - low.1 + 1);
        let count = self.sorted.len() + self.oversize.len();
        if area > i64::try_from(count).expect("body count fits") {
            for &(_, bucket) in &self.ordered_cells {
                let cell = &self.cells[bucket];
                for circle in &self.sorted[cell.start..cell.start + cell.count] {
                    if accepts(circle) {
                        visit(circle);
                    }
                }
            }
        } else {
            for y in low.1..=high.1 {
                for x in low.0..=high.0 {
                    let cell = &self.cells[self.bucket((x, y))];
                    if cell.key != Some((x, y)) {
                        continue;
                    }
                    for circle in &self.sorted[cell.start..cell.start + cell.count] {
                        if accepts(circle) {
                            visit(circle);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Up to k nearest centres, with logical slot breaking distance ties.
    /// Caller-owned output retains its allocation between queries.
    ///
    /// # Errors
    /// Invalid query coordinates or maximum distance.
    pub fn nearest(
        &self,
        centre: Vector,
        distance: Number,
        k: usize,
        output: &mut Vec<(Number, u32)>,
    ) -> Result<(), QueryError> {
        output.clear();
        self.overlap(centre, distance, |circle| {
            let dx = centre.x - circle.centre.x;
            let dy = centre.y - circle.centre.y;
            let squared = dx * dx + dy * dy;
            let in_range = Shape::Circle {
                centre,
                radius: distance,
            }
            .intersects(&Circle {
                radius: Number::ZERO,
                ..*circle
            });
            if !in_range || k == 0 {
                return;
            }
            let position = output.partition_point(|&(prior, slot)| {
                prior < squared || (prior == squared && slot < circle.slot)
            });
            if position < k {
                output.insert(position, (squared, circle.slot));
                output.truncate(k);
            }
        })
    }
}
