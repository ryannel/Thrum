//! Ground wetness delivered to the forward surface and water's film.
//! Simulation owns every store and hollow row. The texture is just their render projection.

use thrum_render::wetness::WetnessProvider;
use thrum_water_model::{CanopyGrid, NO_HOLLOW};

const ROW_WIDTH: u32 = 256;
const MAX_CLASSES: usize = 8;
const HEADER: usize = 7;
const CLASS_TEXELS: usize = 4;
const HOLLOW_START: usize = HEADER + MAX_CLASSES * CLASS_TEXELS;

pub(super) const SOURCE: &str = include_str!("wet_ground.wgsl");

/// Relief height pattern in world metres. A zero period gives unpatterned relief.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReliefPattern {
    /// Distance between the low parts of the relief on each axis. A zero
    /// component disables that axis.
    pub period: [f32; 2],
    /// Distance from a low part to the relief's full height.
    pub bank: f32,
    /// Bottom and top elevations in metres over which the authored relief
    /// loses standing-film cover. Equal bounds disable the elevation mask.
    pub elevation: [f32; 2],
}

/// Appearance and relief of one ground class.
#[derive(Clone, Copy, Debug, Default)]
pub struct GroundClass {
    /// Metres of surface water retained on level ground.
    pub relief: f32,
    /// Slope at which the relief sheds all film.
    pub steepest: f32,
    /// Share of the ground's light the film returns.
    pub share: f32,
    /// Material porosity of the ground, or a negative value to derive it from the material.
    pub porosity: f32,
    /// Roughness of the film.
    pub film_roughness: f32,
    /// World-aligned pattern shared with the authored relief.
    pub pattern: ReliefPattern,
}

/// One class's surface depth and pore fill at the open, half and quarter reaches.
pub type Reaches = [[f32; 2]; 3];

/// One hollow's wetness state at the current tick.
#[derive(Clone, Copy, Debug, Default)]
pub struct HollowWetness {
    /// Current holder's water level in world metres.
    pub level: f32,
    /// Height of the boundary between held and open ground.
    pub mark: f32,
    /// Height of the damp band above the level.
    pub margin: f32,
    /// Fill of the pores under the mark.
    pub held_fill: f32,
}

/// One cell of the prepared hollows ground.
#[derive(Clone, Copy, Debug)]
pub struct GroundCell {
    /// Metres of the bed in world coordinates.
    pub bed: f32,
    /// Hollow label or `NO_HOLLOW` if the cell holds no puddle.
    pub hollow: u32,
    /// Lowest bed height in this cell's hollow.
    pub pit: f32,
    /// Class table index.
    pub class: u8,
}

/// Prepared lattice of the ground receiving rain film.
#[derive(Clone, Debug)]
pub struct HollowsGround {
    /// X and Z of the lattice's lower corner.
    pub origin: [f32; 2],
    /// Cell spacing in metres.
    pub spacing: f32,
    /// Width and height in cells.
    pub size: [u32; 2],
    /// Ground cells in row order.
    pub cells: Vec<GroundCell>,
}

/// Extent and state of a shore, with the same wave support as its water body.
#[derive(Clone, Copy, Debug)]
pub struct ShoreWetness {
    /// The bounds of the shore in world X/Z: minimum then maximum.
    pub bounds: [[f32; 2]; 2],
    /// Still water level in metres.
    pub level: f32,
    /// Height of wave support above still water in metres.
    pub swash: f32,
    /// Damp band above wave support in metres.
    pub margin: f32,
}

/// Render projection of a game's ground stores and prepared ground.
#[derive(Clone, Debug)]
pub struct WetGround {
    provider: WetnessProvider,
    rows: Vec<[f32; 4]>,
    class_count: usize,
    hollow_count: usize,
}

impl WetGround {
    /// Prepare a wetness field. The first class is used off the hollows ground
    /// and on props. Returns none for an invalid table or lattice.
    #[must_use]
    pub fn new(
        canopy: Option<CanopyGrid>,
        ground: Option<HollowsGround>,
        classes: &[GroundClass],
        hollow_count: usize,
    ) -> Option<Self> {
        let width = ROW_WIDTH as usize;
        if classes.is_empty() || classes.len() > MAX_CLASSES || hollow_count >= 1 << 24 {
            return None;
        }
        let mut rows = vec![[0.0; 4]; (HOLLOW_START + hollow_count).next_multiple_of(width)];
        let mut static_rows = Vec::new();
        if let Some(canopy) = canopy {
            append_canopy(canopy, &mut rows, &mut static_rows)?;
        }
        if let Some(ground) = ground {
            if ground.cells.len()
                != (ground.size[0] as usize).checked_mul(ground.size[1] as usize)?
                || ground.spacing <= 0.0
                || ground.cells.iter().any(|cell| {
                    usize::from(cell.class) >= classes.len()
                        || (cell.hollow != NO_HOLLOW && cell.hollow as usize >= hollow_count)
                })
            {
                return None;
            }
            rows[2] = [
                ground.origin[0],
                ground.origin[1],
                ground.spacing,
                ground.size[0] as f32,
            ];
            let offset = rows.len() + static_rows.len();
            rows[3] = [offset as f32, ground.size[1] as f32, 0.0, 0.0];
            static_rows.extend(ground.cells.into_iter().map(|cell| {
                let label = if cell.hollow == NO_HOLLOW {
                    -1.0
                } else {
                    cell.hollow as f32
                };
                [cell.bed, label, f32::from(cell.class), cell.pit]
            }));
        }
        if rows.len() + static_rows.len() >= 1 << 24 {
            return None;
        }
        static_rows.resize(static_rows.len().next_multiple_of(width), [0.0; 4]);
        let mut wet = Self {
            provider: WetnessProvider::new(SOURCE, ROW_WIDTH, rows.clone(), static_rows)?,
            rows,
            class_count: classes.len(),
            hollow_count,
        };
        let dry = vec![[[0.0; 2]; 3]; classes.len()];
        let hollows = vec![HollowWetness::default(); hollow_count];
        wet.publish(classes, &dry, &hollows).then_some(wet)
    }

    /// The wetness provider to register with the scene.
    #[must_use]
    pub fn provider(&self) -> WetnessProvider {
        self.provider.clone()
    }

    /// Set or clear the paired shore field before publishing the tick.
    pub fn set_shore(&mut self, shore: Option<ShoreWetness>) {
        let (bounds, stage) = shore.map_or(([0.0; 4], [0.0; 4]), |shore| {
            let [[xmin, zmin], [xmax, zmax]] = shore.bounds;
            (
                [xmin, zmin, xmax, zmax],
                [shore.level, shore.swash, shore.margin, 1.0],
            )
        });
        self.rows[4] = bounds;
        self.rows[5] = stage;
    }

    /// Publish the stores and hollows for one tick. Returns false when the
    /// number of classes or hollows differs from preparation.
    pub fn publish(
        &mut self,
        classes: &[GroundClass],
        stores: &[Reaches],
        hollows: &[HollowWetness],
    ) -> bool {
        if stores.len() != self.class_count
            || classes.len() != self.class_count
            || hollows.len() != self.hollow_count
        {
            return false;
        }
        for (index, (class, &[[s0, p0], [s1, p1], [s2, p2]])) in
            classes.iter().zip(stores).enumerate()
        {
            let i = HEADER + CLASS_TEXELS * index;
            self.rows[i..i + CLASS_TEXELS].copy_from_slice(&[
                [
                    class.relief,
                    class.steepest,
                    class.share,
                    class.pattern.elevation[0],
                ],
                [
                    class.porosity,
                    class.film_roughness,
                    class.pattern.bank,
                    class.pattern.elevation[1],
                ],
                [s0, p0, s1, p1],
                [s2, p2, class.pattern.period[0], class.pattern.period[1]],
            ]);
        }
        for (row, hollow) in self.rows[HOLLOW_START..].iter_mut().zip(hollows) {
            *row = [hollow.level, hollow.mark, hollow.margin, hollow.held_fill];
        }
        self.provider.publish(&self.rows)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "prepared canopy fields in GPU units"
)]
fn append_canopy(
    canopy: CanopyGrid,
    rows: &mut [[f32; 4]],
    atlas: &mut Vec<[f32; 4]>,
) -> Option<()> {
    let [width, height] = canopy.size;
    if canopy.columns.len() != width.checked_mul(height)?
        || !canopy.cell.is_finite()
        || canopy.cell <= 0.0
        || !canopy.edge_width.is_finite()
        || !canopy.origin.iter().all(|x| x.is_finite())
        || canopy.edge_width < 0.0
    {
        return None;
    }
    let mut lower = [width, height];
    let mut upper = [0; 2];
    let mut ceiling = f64::NEG_INFINITY;
    for (index, [[a, b], [c, d]]) in canopy.columns.into_iter().enumerate() {
        atlas.push([a as f32, b as f32, c as f32, d as f32]);
        if b < 1.0 || d < 1.0 {
            let [x, z] = [index % width, index / width];
            lower = [lower[0].min(x), lower[1].min(z)];
            upper = [upper[0].max(x + 1), upper[1].max(z + 1)];
            if b < 1.0 {
                ceiling = ceiling.max(a);
            }
            if d < 1.0 {
                ceiling = ceiling.max(c);
            }
        }
    }
    if ceiling == f64::NEG_INFINITY {
        return Some(());
    }
    rows[0] = [
        ceiling as f32,
        width as f32,
        height as f32,
        rows.len() as f32,
    ];
    rows[1] = [
        canopy.origin[0] as f32,
        canopy.origin[1] as f32,
        canopy.cell as f32,
        canopy.edge_width as f32,
    ];
    let half = 0.5 * canopy.edge_width / canopy.cell;
    rows[6] = [
        (lower[0] as f64 - half) as f32,
        (lower[1] as f64 - half) as f32,
        (upper[0] as f64 + half) as f32,
        (upper[1] as f64 + half) as f32,
    ];
    Some(())
}
