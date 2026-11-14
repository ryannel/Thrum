//! Reusable derived circle grid. Query order is cell row, column, then slot.
use thrum_core::profile::{Number, StateEncode, Vector};

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
    rows: Vec<Circle>,
    sorted: Vec<usize>,
    oversize: Vec<usize>,
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
            rows: Vec::new(),
            sorted: Vec::new(),
            oversize: Vec::new(),
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
        self.rows.clear();
        self.rows.extend_from_slice(circles);
        self.cells.resize(
            (circles.len() * 2).max(1).next_power_of_two(),
            Cell::default(),
        );
        self.cells.fill(Cell::default());
        self.oversize.clear();
        self.maximum_radius = Number::ZERO;
        for (index, circle) in circles.iter().enumerate() {
            if circle.radius > self.size {
                self.oversize.push(index);
                continue;
            }
            self.maximum_radius = self.maximum_radius.max(circle.radius);
            let key = self.coordinates(circle.centre);
            let bucket = self.bucket(key);
            self.cells[bucket].key = Some(key);
            self.cells[bucket].count += 1;
        }
        let mut start = 0;
        for cell in &mut self.cells {
            cell.start = start;
            start += cell.count;
        }
        self.sorted.resize(start, 0);
        for (index, circle) in circles.iter().enumerate() {
            if circle.radius > self.size {
                continue;
            }
            let bucket = self.bucket(self.coordinates(circle.centre));
            let cell = &mut self.cells[bucket];
            self.sorted[cell.start + cell.written] = index;
            cell.written += 1;
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
        mut visit: impl FnMut(&Circle),
    ) -> Result<(), QueryError> {
        if !valid_circle(&Circle {
            slot: 0,
            centre,
            radius,
        }) {
            return Err(QueryError::Query);
        }
        let accepts = |circle: &Circle| {
            let dx = centre.x - circle.centre.x;
            let dy = centre.y - circle.centre.y;
            let sum = radius + circle.radius;
            dx * dx + dy * dy <= sum * sum
        };
        for &index in &self.oversize {
            let circle = &self.rows[index];
            if accepts(circle) {
                visit(circle);
            }
        }
        let reach = radius + self.maximum_radius;
        let low = self.coordinates(Vector {
            x: centre.x - reach,
            y: centre.y - reach,
        });
        let high = self.coordinates(Vector {
            x: centre.x + reach,
            y: centre.y + reach,
        });
        let area = i64::from(high.0 - low.0 + 1) * i64::from(high.1 - low.1 + 1);
        if area > i64::try_from(self.rows.len()).expect("body count fits") {
            let mut found: Vec<_> = self
                .rows
                .iter()
                .filter(|circle| circle.radius <= self.size && accepts(circle))
                .collect();
            found.sort_unstable_by_key(|circle| {
                let (x, y) = self.coordinates(circle.centre);
                (y, x, circle.slot)
            });
            for circle in found {
                visit(circle);
            }
        } else {
            for y in low.1..=high.1 {
                for x in low.0..=high.0 {
                    let cell = &self.cells[self.bucket((x, y))];
                    if cell.key != Some((x, y)) {
                        continue;
                    }
                    for &index in &self.sorted[cell.start..cell.start + cell.count] {
                        let circle = &self.rows[index];
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
            if squared > distance * distance || k == 0 {
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
