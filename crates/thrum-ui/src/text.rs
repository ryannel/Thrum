//! Whole-pixel text layout using only the cooked font table and sheet frames.

use thrum_asset::layout::{FontGlyph, NO_OUTLINE};
use thrum_asset::{FontProduct, FontStrike};

use crate::SpriteRow;

/// Horizontal alignment within the text box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    /// Align to the left edge.
    #[default]
    Left,
    /// Place equal whole-pixel margins on either side, rounding down on the left.
    Centre,
    /// Align to the right edge.
    Right,
}

/// Treatment of the last line when text exceeds its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Elision {
    /// Keep the beginning followed by an ellipsis.
    #[default]
    End,
    /// Keep both ends around an ellipsis.
    Middle,
}

/// Text styling and fitting in output pixels.
#[derive(Clone, Copy, Debug)]
pub struct TextOptions {
    /// Requested authored size in points. For a pixel font, a whole multiple of its size.
    pub points: f32,
    /// Interface scale. An outline font must have an exact strike at this scale.
    pub scale: u32,
    /// Width and height of the text box in output pixels.
    pub extent: [u32; 2],
    /// Horizontal alignment.
    pub alignment: Alignment,
    /// Wrap at the last space that fits on a line.
    pub wrap: bool,
    /// How to shorten the last visible line.
    pub elision: Elision,
    /// Reserve the widest digit's advance for every ASCII digit.
    pub tabular_digits: bool,
    /// Draw outline frames beneath the fill frames.
    pub outlined: bool,
}

/// A request that does not name a cooked text size.
#[derive(Clone, Copy, Debug)]
pub enum TextError {
    /// This scale has no sheet.
    Scale(u32),
    /// This point size was not cooked at the requested scale.
    Size {
        /// Size requested by the game.
        points: f32,
        /// Interface scale requested by the game.
        scale: u32,
    },
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scale(scale) => write!(f, "font scale {scale} was not cooked"),
            Self::Size { points, scale } => {
                write!(f, "font size {points} was not cooked at scale {scale}")
            }
        }
    }
}

impl std::error::Error for TextError {}

/// A glyph's frame indices and pen position relative to the snapped box origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunGlyph {
    /// Fill frame in the run's sheet.
    pub frame: u32,
    /// Optional outline frame in the same sheet.
    pub outline: Option<u32>,
    /// Baseline pen position from the top left of the text box, in output pixels.
    pub pen: [i64; 2],
}

/// Laid-out glyph offsets, rigid when the box moves.
#[derive(Debug)]
pub struct TextRun {
    glyphs: Vec<RunGlyph>,
    missing: usize,
    magnification: u32,
    sheet_scale: u32,
}

impl TextRun {
    /// Lay out text against exactly one cooked font and sheet.
    ///
    /// # Errors
    /// Returns the missing size or scale when the request would require resampling the font.
    pub fn new(font: &FontProduct, text: &str, options: TextOptions) -> Result<Self, TextError> {
        let (strike, magnification) = select(font, options)?;
        let mut layout = Layout {
            strike,
            options,
            magnification,
            glyphs: Vec::new(),
            missing: 0,
        };
        layout.shape(text);
        Ok(Self {
            glyphs: layout.glyphs,
            missing: layout.missing,
            magnification,
            sheet_scale: strike.metrics.scale,
        })
    }

    /// Pen positions and frame indices in reading order.
    pub fn glyphs(&self) -> &[RunGlyph] {
        &self.glyphs
    }

    /// Number of missing Unicode characters in the run before fitting it to the box.
    pub const fn missing(&self) -> usize {
        self.missing
    }

    /// Scale of the sheet the run was laid out against.
    pub const fn sheet_scale(&self) -> u32 {
        self.sheet_scale
    }

    /// Paint at a snapped output-pixel origin. The caller names the slot of the laid-out sheet.
    pub fn paint(
        &self,
        output: &mut crate::PaintListBuilder,
        sheet: u32,
        origin: [f64; 2],
        fill: [f32; 4],
        outline: [f32; 4],
    ) {
        let origin = origin.map(f64::round);
        for row in self.rows(sheet, origin, fill, outline) {
            output.push(row);
        }
    }

    /// Sprite frame rows for a run. The frame mapping converts cooked indices to handles if the
    /// scene uses named frames. For world rows, position and scale may be transformed by the game.
    #[expect(
        clippy::cast_precision_loss,
        reason = "fitted glyph coordinates are bounded by the output extent"
    )]
    pub fn rows(
        &self,
        sheet: u32,
        origin: [f64; 2],
        fill: [f32; 4],
        outline: [f32; 4],
    ) -> impl Iterator<Item = SpriteRow> + '_ {
        let scale = [self.magnification as f32; 2];
        let row = move |glyph: &RunGlyph, frame, tint| {
            let position = std::array::from_fn(|axis| origin[axis] + glyph.pen[axis] as f64);
            let mut row = SpriteRow::new(frame, position);
            row.sheet = sheet;
            row.scale = scale;
            row.tint = tint;
            row
        };
        self.glyphs
            .iter()
            .filter_map(move |glyph| Some(row(glyph, glyph.outline?, outline)))
            .chain(
                self.glyphs
                    .iter()
                    .map(move |glyph| row(glyph, glyph.frame, fill)),
            )
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel magnification is validated as a bounded whole number"
)]
fn select(font: &FontProduct, options: TextOptions) -> Result<(FontStrike<'_>, u32), TextError> {
    if !(1..=3).contains(&options.scale) {
        return Err(TextError::Scale(options.scale));
    }
    let size_error = || TextError::Size {
        points: options.points,
        scale: options.scale,
    };
    if let [size] = font.sizes()
        && size.pixel == 1
    {
        let multiple = options.points / size.points;
        if !multiple.is_finite() || !(1.0..=256.0).contains(&multiple) || multiple.fract() != 0.0 {
            return Err(size_error());
        }
        let strike = font.strike(size.points, 1).ok_or_else(size_error)?;
        return Ok((strike, multiple as u32 * options.scale));
    }
    if font.sheet(options.scale).is_none() {
        return Err(TextError::Scale(options.scale));
    }
    Ok((
        font.strike(options.points, options.scale)
            .ok_or_else(size_error)?,
        1,
    ))
}

#[derive(Clone, Copy)]
struct Glyph {
    ch: char,
    entry: FontGlyph,
    advance: i64,
    bounds: [i64; 4],
}

#[derive(Clone, Copy, Default)]
struct Line {
    tail: Option<char>,
    pen: i64,
    bounds: [i64; 4],
}

impl Line {
    fn width(self) -> i64 {
        self.bounds[2].max(self.pen) - self.bounds[0]
    }
}

struct Layout<'a> {
    strike: FontStrike<'a>,
    options: TextOptions,
    magnification: u32,
    glyphs: Vec<RunGlyph>,
    missing: usize,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "line and frame bounds round outward to whole pixels"
)]
impl Layout<'_> {
    fn shape(&mut self, text: &str) {
        let [width, height] = self.options.extent.map(i64::from);
        let scale = self.magnification as f32;
        let metrics = self.strike.metrics;
        let line_ascent = (metrics.ascent * scale).ceil() as i64;
        let line_descent = (metrics.descent * scale).ceil() as i64;
        let mut paragraphs: Vec<_> = text
            .split('\n')
            .map(|paragraph| self.characters(paragraph))
            .collect();
        // Keep one baseline inset for the run, including accents and strokes that
        // extend above or below the font's nominal line metrics.
        let (ascent, descent) = paragraphs
            .iter()
            .flatten()
            .fold((line_ascent, line_descent), |(ascent, descent), glyph| {
                (ascent.max(-glyph.bounds[1]), descent.max(glyph.bounds[3]))
            });
        let spacing = (ascent + descent + (metrics.gap * scale).round() as i64).max(1);
        let mut baseline = ascent;
        let mut paragraphs = paragraphs.drain(..).peekable();
        while let Some(glyphs) = paragraphs.next() {
            if baseline + descent > height {
                continue;
            }
            let mut rest = glyphs.as_slice();
            loop {
                let (mut fit, mut resume) = self.fit(rest, width, baseline, height);
                let more = resume < rest.len() || paragraphs.peek().is_some();
                let last_line = baseline + spacing + descent > height;
                if fit < rest.len() && (!self.options.wrap || last_line || resume == 0)
                    || more && last_line
                {
                    let shortened = self.elide(rest, width, baseline, height);
                    self.place(&shortened, baseline, width);
                    baseline += spacing;
                    break;
                }
                if fit == rest.len() {
                    resume = fit;
                } else {
                    while fit > 0 && rest[fit - 1].ch == ' ' {
                        fit -= 1;
                    }
                }
                self.place(&rest[..fit], baseline, width);
                baseline += spacing;
                if resume == rest.len() {
                    break;
                }
                rest = &rest[resume..];
            }
        }
    }

    fn characters(&mut self, text: &str) -> Vec<Glyph> {
        let mut characters = text.chars().peekable();
        let mut glyphs = Vec::new();
        while let Some(base) = characters.next() {
            let composed = characters.peek().and_then(|&mark| {
                let compositions = self.strike.compositions;
                let i = compositions
                    .binary_search_by_key(&[u32::from(base), u32::from(mark)], |entry| entry.pair)
                    .ok()?;
                let scalar = char::from_u32(compositions[i].scalar)?;
                self.strike.glyph(scalar).map(|_| scalar)
            });
            if composed.is_some() {
                characters.next();
            }
            let (glyph, missing) = self.glyph(composed.unwrap_or(base));
            self.missing += usize::from(missing);
            glyphs.push(glyph);
        }
        glyphs
    }

    fn glyph(&self, ch: char) -> (Glyph, bool) {
        let entry = self.strike.glyph(ch).filter(|entry| entry.scalar != 0);
        let missing = entry.is_none();
        let mut entry = entry.copied().unwrap_or(self.strike.glyphs[0]);
        if !self.options.outlined {
            entry.outline = NO_OUTLINE;
        }
        let digit = self.options.tabular_digits && ch.is_ascii_digit() && !missing;
        let advance = if digit {
            self.strike.metrics.widest_digit
        } else {
            entry.advance
        };
        let mut bounds = [0; 4];
        for frame in [entry.frame, entry.outline]
            .into_iter()
            .filter_map(|index| self.strike.frame(index))
        {
            let trim = std::array::from_fn::<_, 2, _>(|axis| {
                f64::from(frame.trim[axis]) - f64::from(frame.pivot[axis])
            });
            let scale = f64::from(self.magnification);
            for axis in 0..2 {
                bounds[axis] = bounds[axis].min((trim[axis] * scale).floor() as i64);
                bounds[axis + 2] = bounds[axis + 2]
                    .max(((trim[axis] + f64::from(frame.size[axis])) * scale).ceil() as i64);
            }
        }
        (
            Glyph {
                ch,
                entry,
                advance: i64::from(advance) * i64::from(self.magnification),
                bounds,
            },
            missing,
        )
    }

    fn kerning(&self, first: Option<char>, second: char) -> i64 {
        let Some(first) = first else { return 0 };
        if self.options.tabular_digits && (first.is_ascii_digit() || second.is_ascii_digit()) {
            return 0;
        }
        let pair = [u32::from(first), u32::from(second)];
        let table = self.strike.kerning;
        table
            .binary_search_by_key(&pair, |entry| entry.pair)
            .map_or(0, |index| {
                i64::from(table[index].pixels) * i64::from(self.magnification)
            })
    }

    fn append(&self, mut line: Line, glyph: Glyph) -> (Line, i64) {
        let pen = line.pen + self.kerning(line.tail, glyph.ch);
        let [left, top, right, bottom] = glyph.bounds;
        line.bounds[0] = line.bounds[0].min(pen + left);
        line.bounds[1] = line.bounds[1].min(top);
        line.bounds[2] = line.bounds[2].max(pen + right);
        line.bounds[3] = line.bounds[3].max(bottom);
        line.tail = Some(glyph.ch);
        line.pen = pen + glyph.advance;
        (line, pen)
    }

    fn fits(line: Line, width: i64, baseline: i64, height: i64) -> bool {
        line.width() <= width
            && baseline + line.bounds[1] >= 0
            && baseline + line.bounds[3] <= height
    }

    fn fit(&self, glyphs: &[Glyph], width: i64, baseline: i64, height: i64) -> (usize, usize) {
        let mut line = Line::default();
        let mut space = None;
        for (index, &glyph) in glyphs.iter().enumerate() {
            if glyph.ch == ' ' && index > 0 {
                space = Some(index);
            }
            line = self.append(line, glyph).0;
            if !Self::fits(line, width, baseline, height) {
                return if self.options.wrap
                    && let Some(space) = space
                {
                    (space, space + 1)
                } else {
                    (index, 0)
                };
            }
        }
        (glyphs.len(), glyphs.len())
    }

    fn elide(&self, glyphs: &[Glyph], width: i64, baseline: i64, height: i64) -> Vec<Glyph> {
        let ellipsis = self.glyph('…').0;
        let middle = self.options.elision == Elision::Middle;
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut start = 0;
        let mut end = glyphs.len();
        let mut prefix = Line::default();
        let mut suffix = Line::default();
        let mut next = None;
        while start < end {
            let append_left = !middle || left.len() <= right.len();
            let (head, tail, next_ch) = if append_left {
                (self.append(prefix, glyphs[start]).0, suffix, next)
            } else {
                let item = glyphs[end - 1];
                (prefix, self.prepend(item, suffix, next), Some(item.ch))
            };
            let mark = self.append(head, ellipsis).0;
            let joined = self.join(mark, tail, next_ch);
            if !Self::fits(joined, width, baseline, height) {
                break;
            }
            prefix = head;
            suffix = tail;
            next = next_ch;
            if append_left {
                left.push(glyphs[start]);
                start += 1;
            } else {
                end -= 1;
                right.push(glyphs[end]);
            }
        }
        if Self::fits(
            self.append(Line::default(), ellipsis).0,
            width,
            baseline,
            height,
        ) {
            left.push(ellipsis);
            left.extend(right.into_iter().rev());
        }
        left
    }

    fn prepend(&self, item: Glyph, tail: Line, next: Option<char>) -> Line {
        self.join(self.append(Line::default(), item).0, tail, next)
    }

    fn join(&self, mut head: Line, tail: Line, next: Option<char>) -> Line {
        if let Some(next) = next {
            let offset = head.pen + self.kerning(head.tail, next);
            head.pen = offset + tail.pen;
            head.bounds[0] = head.bounds[0].min(offset + tail.bounds[0]);
            head.bounds[2] = head.bounds[2].max(offset + tail.bounds[2]);
            head.bounds[1] = head.bounds[1].min(tail.bounds[1]);
            head.bounds[3] = head.bounds[3].max(tail.bounds[3]);
            head.tail = tail.tail;
        }
        head
    }

    fn place(&mut self, glyphs: &[Glyph], baseline: i64, width: i64) {
        let start = self.glyphs.len();
        let mut line = Line::default();
        for &glyph in glyphs {
            let (updated, pen) = self.append(line, glyph);
            self.glyphs.push(RunGlyph {
                frame: glyph.entry.frame,
                outline: (glyph.entry.outline != NO_OUTLINE).then_some(glyph.entry.outline),
                pen: [pen, baseline],
            });
            line = updated;
        }
        let margin = width - line.width();
        let align = match self.options.alignment {
            Alignment::Left => 0,
            Alignment::Centre => margin / 2,
            Alignment::Right => margin,
        };
        for item in &mut self.glyphs[start..] {
            item.pen[0] += align - line.bounds[0];
        }
    }
}
