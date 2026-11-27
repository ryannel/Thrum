//! Ground wetness delivered to the forward surface and water's film.
//! Simulation owns every store and hollow row. The texture is just their render projection.

use thrum_render::wetness::WetnessProvider;
use thrum_water_model::{CanopyGrid, Map, NO_HOLLOW};

mod shore;
use shore::ShoreLayout;

const ROW_WIDTH: u32 = 256;
const MAX_CLASSES: usize = 8;
const HEADER: usize = 7;
const CLASS_TEXELS: usize = 4;
const HOLLOW_START: usize = HEADER + MAX_CLASSES * CLASS_TEXELS;
const ADMISSION_ROW: usize = HEADER + 3;

pub(super) const BED_REACHES: &str = include_str!("bed_reaches.wgsl");
pub(super) const SOURCE: &str = concat!(
    include_str!("bed_reaches.wgsl"),
    include_str!("canopy.wgsl"),
    include_str!("wet_ground.wgsl"),
);

/// One authored, repeating relief texture, mapped to world X/Z.
#[derive(Clone, Copy, Debug)]
pub struct ReliefTile {
    /// World position of the texture's UV origin.
    pub origin: [f32; 2],
    /// World metres covered by one repeat on each axis.
    pub period: [f32; 2],
    /// Width and height in texels.
    pub size: [u32; 2],
    /// Row-major normalised heights, encoded as UNORM8.
    pub heights: &'static [u8],
}

/// A ground class's authored relief and elevation bounds.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReliefPattern {
    /// One texture shared with the ground's material and bed.
    pub tile: Option<ReliefTile>,
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
    /// Current holder's water level in metres from the field's vertical datum.
    pub level: f32,
    /// Height of the boundary between held and open ground, from the same datum.
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
    ground_classes: u8,
    relief_addresses: [u32; MAX_CLASSES],
    hollow_pits: Vec<f32>,
    datum: f32,
    shore_layout: Option<ShoreLayout>,
    shore_revision: Option<u64>,
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
        Self::prepare(canopy, ground, classes, hollow_count, None, 0.0, 0.0)
    }

    /// Prepare high ground whose thin film uses the forward material lobe.
    /// Dynamic hollow levels and marks are relative to the vertical datum.
    /// The static bed, pits and canopy stay in world metres. A positive
    /// canopy half-width smooths the sparse canopy vertically for material shading.
    #[must_use]
    pub fn new_forward_at_datum(
        canopy: Option<CanopyGrid>,
        ground: HollowsGround,
        classes: &[GroundClass],
        hollow_count: usize,
        datum: f32,
        canopy_half_width: f32,
    ) -> Option<Self> {
        let mut field = Self::prepare(
            canopy,
            Some(ground),
            classes,
            hollow_count,
            None,
            datum,
            canopy_half_width,
        )?;
        field.ground_classes = 0;
        Some(field)
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "prepared shore lattice uses GPU metres"
    )]
    fn prepare(
        canopy: Option<CanopyGrid>,
        ground: Option<HollowsGround>,
        classes: &[GroundClass],
        hollow_count: usize,
        shore: Option<(&Map, [f32; 4])>,
        datum: f32,
        canopy_half_width: f32,
    ) -> Option<Self> {
        let row_width = if canopy
            .as_ref()
            .is_some_and(|grid| !grid.sparse_cells.is_empty())
        {
            1024
        } else {
            ROW_WIDTH
        };
        let width = row_width as usize;
        if classes.is_empty()
            || classes.len() > MAX_CLASSES
            || hollow_count >= 1 << 24
            || !datum.is_finite()
        {
            return None;
        }
        let shore_layout = shore.map(|(map, wave_heights)| ShoreLayout {
            dynamic: HOLLOW_START + hollow_count,
            cells: map.bed().heights().len(),
            wave_heights,
        });
        let rows_end =
            HOLLOW_START + hollow_count + shore_layout.as_ref().map_or(0, ShoreLayout::row_count);
        let mut rows = vec![[0.0; 4]; rows_end.next_multiple_of(width)];
        let mut static_rows = Vec::new();
        if let Some(((map, _), layout)) = shore.zip(shore_layout.as_ref()) {
            let (origin, spacing, size) = map.bed().lattice();
            rows[4] = [
                origin[0] as f32,
                origin[1] as f32,
                spacing as f32,
                size[0] as f32,
            ];
            rows[5] = [
                size[1] as f32,
                layout.dynamic as f32,
                rows.len() as f32,
                2.0,
            ];
            static_rows.extend(map.bed().heights().chunks(4).map(|chunk| {
                let mut pack = [0.0; 4];
                for (slot, &height) in pack.iter_mut().zip(chunk) {
                    *slot = height as f32;
                }
                pack
            }));
        }
        let mut ground_classes = 0;
        let mut hollow_pits = vec![f32::INFINITY; hollow_count];
        if let Some(canopy) = canopy {
            append_canopy(canopy, &mut rows, &mut static_rows, canopy_half_width)?;
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
            let (low, high) = ground
                .cells
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), cell| {
                    (lo.min(cell.bed), hi.max(cell.bed))
                });
            rows[3] = [offset as f32, ground.size[1] as f32, low, high];
            static_rows.extend(ground.cells.into_iter().map(|cell| {
                ground_classes |= 1 << cell.class;
                let label = if cell.hollow == NO_HOLLOW {
                    -1.0
                } else {
                    let pit = &mut hollow_pits[cell.hollow as usize];
                    *pit = pit.min(cell.pit - datum);
                    cell.hollow as f32
                };
                [cell.bed, label, f32::from(cell.class), cell.pit]
            }));
        }
        let mut relief_addresses = [0; MAX_CLASSES];
        for (address, class) in relief_addresses.iter_mut().zip(classes) {
            if let Some(tile) = class.pattern.tile {
                *address = append_relief(tile, rows.len(), &mut static_rows)?;
            }
        }
        if rows.len() + static_rows.len() >= 1 << 24 {
            return None;
        }
        static_rows.resize(static_rows.len().next_multiple_of(width), [0.0; 4]);
        let mut wet = Self {
            provider: WetnessProvider::new(SOURCE, row_width, rows.clone(), static_rows)?,
            rows,
            class_count: classes.len(),
            hollow_count,
            ground_classes,
            relief_addresses,
            hollow_pits,
            datum,
            shore_layout,
            shore_revision: None,
        };
        let dry = vec![[[0.0; 2]; 3]; classes.len()];
        let hollows = vec![
            HollowWetness {
                level: -datum,
                mark: -datum,
                ..HollowWetness::default()
            };
            hollow_count
        ];
        wet.publish(classes, &dry, &hollows).then_some(wet)
    }

    /// The wetness provider to register with the scene.
    #[must_use]
    pub fn provider(&self) -> WetnessProvider {
        self.provider.clone()
    }

    /// Set or clear a rectangular shore on a field prepared with [Self::new]
    /// before publishing the tick.
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
                    self.relief_addresses[index] as f32,
                    class.pattern.elevation[1],
                ],
                [s0, p0, s1, p1],
                [s2, p2, 0.0, 0.0],
            ]);
        }
        for (row, hollow) in self.rows[HOLLOW_START..].iter_mut().zip(hollows) {
            *row = [hollow.level, hollow.mark, hollow.margin, hollow.held_fill];
        }
        self.rows[ADMISSION_ROW][2] =
            f32::from(u8::from(classes.iter().zip(stores).enumerate().any(
                |(index, (class, reaches))| {
                    self.ground_classes & (1 << index) != 0
                        && class.relief > 0.0
                        && reaches.iter().any(|reach| reach[0] > 0.0)
                },
            )));
        self.rows[ADMISSION_ROW][3] = self.datum;
        let active = self.rows[5][3] > 0.0
            || stores.iter().flatten().flatten().any(|&value| value > 0.0)
            || hollows.iter().zip(&self.hollow_pits).any(|(hollow, &pit)| {
                hollow.level > pit || hollow.margin > 0.0 || hollow.held_fill > 0.0
            });
        self.provider.publish_state(&self.rows, active)
    }
}

fn append_relief(tile: ReliefTile, base: usize, atlas: &mut Vec<[f32; 4]>) -> Option<u32> {
    let [width, height] = tile.size.map(|v| v as usize);
    if width == 0
        || height == 0
        || tile.heights.len() != width.checked_mul(height)?
        || !tile.origin.into_iter().all(f32::is_finite)
        || !tile.period.into_iter().all(|v| v.is_finite() && v > 0.0)
    {
        return None;
    }
    let address = u32::try_from(base + atlas.len()).ok()?;
    atlas.push([
        tile.origin[0],
        tile.origin[1],
        tile.period[0],
        tile.period[1],
    ]);
    atlas.push([width as f32, height as f32, 0.0, 0.0]);
    atlas.extend(tile.heights.chunks(4).map(|pack| {
        let mut values = [0.0; 4];
        for (value, height) in values.iter_mut().zip(pack) {
            *value = f32::from(*height) / 255.0;
        }
        values
    }));
    Some(address)
}

pub(super) fn film_bounds(
    wetness: &thrum_render::wetness::PreparedWetness,
) -> Option<crate::optics::BodyBounds> {
    wetness.read_rows(|rows| {
        if rows[ADMISSION_ROW][2] == 0.0 {
            return None;
        }
        let [x, z, spacing, width] = rows[2];
        let [_, height, low, high] = rows[3];
        let tolerance = 0.028 + 0.5 * spacing;
        crate::optics::BodyBounds::new(
            [x, low - tolerance, z],
            [x + spacing * width, high + tolerance, z + spacing * height],
        )
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "prepared canopy fields in GPU units"
)]
fn append_canopy(
    canopy: CanopyGrid,
    rows: &mut [[f32; 4]],
    atlas: &mut Vec<[f32; 4]>,
    half_width: f32,
) -> Option<()> {
    let [width, height] = canopy.size;
    let sparse = !canopy.sparse_cells.is_empty();
    if (!sparse && canopy.columns.len() != width.checked_mul(height)?)
        || !canopy.cell.is_finite()
        || canopy.cell <= 0.0
        || !canopy.edge_width.is_finite()
        || !canopy.origin.iter().all(|x| x.is_finite())
        || canopy.edge_width < 0.0
        || !half_width.is_finite()
        || half_width < 0.0
        || (!sparse && half_width > 0.0 && width > 0 && height > 0)
    {
        return None;
    }
    let mut lower = [width, height];
    let mut upper = [0; 2];
    let mut ceiling = f64::NEG_INFINITY;
    for (index, &[[a, b], [c, d]]) in canopy.columns.iter().enumerate() {
        if b < 1.0 || d < 1.0 {
            let [x, z] = if sparse {
                canopy.sparse_cells.get(index)?.map(|value| value as usize)
            } else {
                [index % width, index / width]
            };
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
    let (origin, offset) = crate::canopy::origin_parts(&canopy);
    if sparse {
        let columns = crate::canopy::SparseColumns::pack(&canopy)?;
        atlas.push([columns.code as f32, offset[0], offset[1], half_width]);
        atlas.extend(columns.texels);
    } else {
        atlas.extend(canopy.columns.into_iter().map(crate::canopy::layers));
    }
    rows[0] = [
        crate::canopy::ceiling(ceiling + f64::from(half_width)),
        width as f32,
        if sparse {
            -(height as f32)
        } else {
            height as f32
        },
        rows.len() as f32,
    ];
    rows[1] = [
        origin[0],
        origin[1],
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
