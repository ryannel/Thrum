//! CPU composition of pixel-kind sheets over a submitted SDR background.

use super as status;
use thrum_asset::{AssetStore, FontProduct, Profile, SheetProduct};
use thrum_render::sprite::EmbeddedSheet;
use thrum_ui::{Font, Image, PaintInputs, SpriteRow, Viewport};

/// Output-space reference pixels and their blending tolerance.
pub struct Reference {
    extent: [u32; 2],
    rgba: Vec<u8>,
    blended: Vec<bool>,
}

#[derive(Clone, Copy)]
/// Pixel-kind input from the embedded or cooked sheets.
pub enum Sheet<'a> {
    /// Embedded authored frame rectangles.
    Embedded(&'a EmbeddedSheet),
    /// Validated sheet product and its source texels.
    Cooked(&'a SheetProduct),
}

struct Frame<'a> {
    extent: [u32; 2],
    size: [u32; 2],
    offset: [f64; 2],
    texel: [u32; 2],
    pixels: &'a [u8],
}

impl<'a> Sheet<'a> {
    fn frame(self, frame: u32) -> Frame<'a> {
        match self {
            Self::Embedded(sheet) => {
                let rect = sheet.frames[frame as usize];
                Frame {
                    extent: sheet.extent,
                    size: rect.size,
                    offset: [0.0; 2],
                    texel: rect.origin,
                    pixels: &sheet.rgba,
                }
            }
            Self::Cooked(sheet) => {
                let frame = sheet.frame_at(frame).expect("cooked frame");
                let texture = sheet.texture();
                let level = &texture.levels()[0];
                Frame {
                    extent: [level.width, level.height],
                    size: frame.size,
                    offset: std::array::from_fn(|axis| {
                        f64::from(frame.trim[axis]) - f64::from(frame.pivot[axis])
                    }),
                    texel: [frame.origin[0], frame.origin[1] + frame.page * level.height],
                    pixels: texture.level_data(level),
                }
            }
        }
    }
}

impl Reference {
    /// Start over captured background pixels.
    /// # Panics
    /// Panics if the RGBA byte count differs from the extent.
    pub fn new(extent: [u32; 2], rgba: Vec<u8>) -> Self {
        assert_eq!(rgba.len(), (extent[0] * extent[1] * 4) as usize);
        Self {
            extent,
            blended: vec![false; rgba.len() / 4],
            rgba,
        }
    }

    /// Locate the first pixel outside the CPU reference tolerance.
    /// # Panics
    /// Panics for a mismatched RGBA length or an out-of-range comparison region.
    pub fn first_difference(&self, actual: &[u8], region: [[u32; 2]; 2]) -> Option<[u32; 4]> {
        assert_eq!(self.rgba.len(), actual.len());
        let [[left, top], [right, bottom]] = region;
        for y in top..bottom {
            for x in left..right {
                let index = (y * self.extent[0] + x) as usize;
                for channel in 0..4 {
                    let at = index * 4 + channel;
                    let tolerance = u8::from(channel < 3 && self.blended[index]);
                    if self.rgba[at].abs_diff(actual[at]) > tolerance {
                        return Some([x, y, self.rgba[at].into(), actual[at].into()]);
                    }
                }
            }
        }
        None
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "test coordinates refer to bounded whole output pixels and sheet texels"
    )]
    /// Composite one row in linear light from its source frame.
    /// # Panics
    /// Panics if the supplied sheet lacks the row frame.
    pub fn draw(&mut self, row: SpriteRow, sheet: Sheet<'_>) {
        let frame = sheet.frame(row.frame);
        let scale = row.scale.map(f64::from);
        let start: [f64; 2] =
            std::array::from_fn(|axis| row.position[axis] + scale[axis] * frame.offset[axis]);
        let end: [f64; 2] =
            std::array::from_fn(|axis| start[axis] + scale[axis] * f64::from(frame.size[axis]));
        for y in start[1].round().max(0.0) as u32..(end[1].round() as u32).min(self.extent[1]) {
            for x in start[0].round().max(0.0) as u32..(end[0].round() as u32).min(self.extent[0]) {
                let source: [u32; 2] = std::array::from_fn(|axis| {
                    let local = (f64::from([x, y][axis]) + 0.5 - start[axis]) / scale[axis];
                    frame.texel[axis] + (local as u32).min(frame.size[axis] - 1)
                });
                let at = ((source[1] * frame.extent[0] + source[0]) * 4) as usize;
                let rgba = &frame.pixels[at..at + 4];
                let alpha = f64::from(rgba[3]) / 255.0 * f64::from(row.tint[3]);
                if alpha == 0.0 {
                    continue;
                }
                let pixel = (y * self.extent[0] + x) as usize;
                self.blended[pixel] = alpha < 1.0;
                for (channel, &source) in rgba[..3].iter().enumerate() {
                    let at = pixel * 4 + channel;
                    let output =
                        decode(source) * f64::from(row.tint[channel]) * f64::from(row.tint[3])
                            + decode(self.rgba[at]) * (1.0 - alpha);
                    self.rgba[at] = encode(output);
                }
                self.rgba[pixel * 4 + 3] = (alpha * 255.0
                    + f64::from(self.rgba[pixel * 4 + 3]) * (1.0 - alpha))
                    .round() as u8;
            }
        }
    }

    /// Extract RGBA and one blending-tolerance byte per pixel.
    /// # Panics
    /// Panics for an out-of-range region.
    pub fn crop(&self, [[left, top], [right, bottom]]: [[u32; 2]; 2]) -> (Vec<u8>, Vec<u8>) {
        let mut rgba = Vec::new();
        let mut tolerance = Vec::new();
        for y in top..bottom {
            let start = (y * self.extent[0] + left) as usize;
            let end = start + (right - left) as usize;
            rgba.extend_from_slice(&self.rgba[start * 4..end * 4]);
            tolerance.extend(
                self.blended[start..end]
                    .iter()
                    .map(|&blended| u8::from(blended)),
            );
        }
        (rgba, tolerance)
    }
}

fn decode(code: u8) -> f64 {
    let x = f64::from(code) / 255.0;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "linear colour is clamped to the normalized output range"
)]
fn encode(linear: f64) -> u8 {
    let x = linear.clamp(0.0, 1.0);
    let x = if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    (x * 255.0).round() as u8
}

/// Compose a fresh status layout independently of the render stage.
/// # Panics
/// Panics if the proving assets or viewport cannot draw the complete display.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the status boxes are clipped to the viewport and snapped to whole pixels"
)]
pub fn compose(
    viewport: Viewport,
    tick: u64,
    font: &FontProduct,
    art: &SheetProduct,
) -> (Reference, [[u32; 2]; 2]) {
    let mut tree = status::tree(Image {
        sheet: 2,
        frame: 0,
        texels: art.frame_at(0).expect("health badge").size,
    })
    .expect("fresh status layout");
    status::update(&mut tree, tick);
    let paint = tree
        .frame(
            viewport,
            PaintInputs {
                stamp: None,
                fonts: &[Font {
                    product: font,
                    sheet: 1,
                }],
                white: status::WHITE_TILE,
            },
        )
        .expect("fresh status paint");
    assert_eq!(tree.elisions().count(), 0, "reference status text fits");
    assert_eq!(tree.overflows().count(), 0, "reference status boxes fit");
    let bounds = tree.bounds(1).expect("status panel");
    let region = [bounds.origin, bounds.end()]
        .map(|edge| edge.map(|v| (v * f64::from(viewport.scale)).round() as u32));
    let panel = status::panel_sheet();
    let sheets = [
        Sheet::Embedded(&panel.sheet),
        Sheet::Cooked(font.sheet(viewport.scale).expect("font strike sheet")),
        Sheet::Cooked(art),
    ];
    let mut reference = Reference::new(
        viewport.extent,
        vec![0; (viewport.extent[0] * viewport.extent[1] * 4) as usize],
    );
    for &row in paint.rows.iter() {
        reference.draw(row, sheets[row.sheet as usize]);
    }
    (reference, region)
}

/// Open the cooker products used by the status oracle.
/// # Panics
/// Panics if the proving store lacks either validated product.
pub fn open(store: &AssetStore) -> (FontProduct, SheetProduct) {
    let head = store
        .read_head()
        .expect("HEAD")
        .expect("cooked publication");
    let manifest = store.load_manifest(&head).expect("manifest");
    let product = |name| {
        let entry = manifest
            .resolve(name, &[Profile::Portable])
            .expect("cooked status product");
        store.map(&entry.product).expect("verified product")
    };
    (
        FontProduct::open(product("type")).expect("font"),
        SheetProduct::open(product("status")).expect("art sheet"),
    )
}
