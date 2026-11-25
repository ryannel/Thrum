//! Cooked type specimens over the V1 sprite field.

use std::sync::Arc;

use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedSource, FontCandidate, FontScale, ProductLease,
};
use thrum_render::runtime::FrameError;
use thrum_ui::{
    Alignment, Elision, InterfaceConfig, InterfaceSheets, PaintList, PaintListBuilder, PaintStamp,
    SpriteRow, TextOptions, TextRun,
};

use crate::interface_card::Card;

const TYPE: &str = "type";
const PIXEL: &str = "pixel";
const WHITE: [f32; 4] = [1.0; 4];
const DARK: [f32; 4] = [0.015, 0.024, 0.041, 1.0];

/// Text cards of the proving scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceText {
    /// Light, dark and outlined text with European Latin samples.
    Type,
    /// Rigid, right-aligned counters and a slowly drifting label.
    Counting,
    /// Small specimen with known glyph strings for submitted-image comparisons.
    Specimen,
}

pub(crate) struct FontCard {
    mode: InterfaceText,
    lease: ProductLease,
    scale: FontScale,
    active: Option<Layout>,
    prepared: Option<Layout>,
}

struct Layout {
    stamp: PaintStamp,
    extent: [u32; 2],
    value: u64,
    paint: PaintList,
}

impl FontCard {
    pub(crate) fn attach(
        source: CookedSource,
        mode: InterfaceText,
        scale: u8,
    ) -> Result<(Self, CookedAssets), FrameError> {
        let mut config = CookedConfig::new(source, Vec::new());
        for name in [TYPE, PIXEL] {
            config.lease_font(name);
        }
        if mode == InterfaceText::Specimen {
            config.upload_budget = thrum_asset_gpu::COPY_UNIT;
        }
        let request = config.font_scale_request();
        if !request.set(u32::from(scale)) {
            return Err(FrameError::InvalidState(
                "interface scale must be 1, 2 or 3",
            ));
        }
        let assets =
            CookedAssets::new(config).map_err(|error| FrameError::Scene(error.to_string()))?;
        Ok((
            Self {
                mode,
                lease: assets.lease(),
                scale: request,
                active: None,
                prepared: None,
            },
            assets,
        ))
    }

    pub(crate) fn config(&self) -> InterfaceConfig {
        let mut config = Card::new(1, None, true).config();
        config.row_capacity = 2048;
        config.cooked = Some(InterfaceSheets {
            names: [TYPE, PIXEL].map(str::to_owned).into(),
            source: Arc::new(self.lease.clone()),
        });
        config
    }

    pub(crate) fn set_scale(&self, scale: u32) -> bool {
        self.scale.set(scale)
    }

    pub(crate) fn paint(&mut self, extent: [u32; 2], tick: u64) -> [Option<PaintList>; 2] {
        let value = match self.mode {
            InterfaceText::Type => 0,
            InterfaceText::Counting => tick,
            InterfaceText::Specimen => tick / 4 % 10,
        };
        let committed = self.lease.committed_font();
        if let Some(committed) = &committed
            && self
                .prepared
                .as_ref()
                .is_some_and(|layout| layout.stamp == committed.stamp)
        {
            self.active = self.prepared.take();
        }
        let active = committed
            .as_ref()
            .and_then(|font| Self::layout(&mut self.active, self.mode, font, extent, value).ok());
        let prepared = if let Some(font) = self.lease.font_candidate() {
            match Self::layout(&mut self.prepared, self.mode, &font, extent, value) {
                Ok(paint) => {
                    self.lease.answer_font(font.stamp, Ok(()));
                    Some(paint)
                }
                Err(reason) => {
                    self.prepared = None;
                    self.lease.answer_font(font.stamp, Err(reason));
                    None
                }
            }
        } else {
            self.prepared = None;
            None
        };
        [active, prepared]
    }

    fn layout(
        cached: &mut Option<Layout>,
        mode: InterfaceText,
        fonts: &FontCandidate,
        extent: [u32; 2],
        value: u64,
    ) -> Result<PaintList, String> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "font scale is restricted to 1..=3"
        )]
        let scale = fonts.stamp.scale as u8;
        if let Some(layout) = cached
            && layout.stamp == fonts.stamp
            && layout.extent == extent
            && layout.value == value
        {
            return Ok(layout.paint.clone());
        }
        let mut card = Specimen {
            fonts,
            scale,
            extent,
            builder: PaintListBuilder::new(extent),
        };
        card.builder.stamp(fonts.stamp);
        match mode {
            InterfaceText::Type => card.type_card()?,
            InterfaceText::Counting => card.counting_card(value)?,
            InterfaceText::Specimen => card.comparison(value)?,
        }
        let paint = card.builder.finish();
        *cached = Some(Layout {
            stamp: fonts.stamp,
            extent,
            value,
            paint: paint.clone(),
        });
        Ok(paint)
    }
}

struct Specimen<'a> {
    fonts: &'a FontCandidate,
    scale: u8,
    extent: [u32; 2],
    builder: PaintListBuilder,
}

#[derive(Clone, Copy)]
struct Text<'a> {
    value: &'a str,
    origin: [f64; 2],
    extent: [u32; 2],
    points: u8,
    tint: [f32; 4],
    kind: &'static str,
    outlined: bool,
    alignment: Alignment,
    tabular: bool,
    wrap: bool,
    elision: Elision,
}

impl<'a> Text<'a> {
    fn at(value: &'a str, origin: [f64; 2], points: u8) -> Self {
        Self {
            value,
            origin,
            points,
            extent: [320, 36],
            tint: WHITE,
            kind: TYPE,
            outlined: false,
            alignment: Alignment::Left,
            tabular: false,
            wrap: false,
            elision: Elision::End,
        }
    }
}

impl Specimen<'_> {
    fn rect(&mut self, origin: [f64; 2], size: [u16; 2], tint: [f32; 4]) {
        let mut row = SpriteRow::new(0, origin.map(|x| x * f64::from(self.scale)));
        row.scale = size.map(|side| f32::from(side) * f32::from(self.scale));
        row.tint = tint;
        self.builder.push(row);
    }

    fn text(&mut self, text: Text<'_>) -> Result<(), String> {
        let font = self
            .fonts
            .font(text.kind)
            .ok_or_else(|| format!("font {} is missing", text.kind))?;
        let origin = text.origin.map(|x| x * f64::from(self.scale));
        let extent = text.extent.map(|size| size * u32::from(self.scale));
        let run = TextRun::new(
            font,
            text.value,
            TextOptions {
                points: f32::from(text.points),
                scale: u32::from(self.scale),
                extent,
                alignment: text.alignment,
                wrap: text.wrap,
                elision: text.elision,
                tabular_digits: text.tabular,
                outlined: text.outlined,
            },
        )
        .map_err(|error| format!("font {}: {error}", text.kind))?;
        run.paint(
            &mut self.builder,
            if text.kind == TYPE { 1 } else { 2 },
            origin,
            text.tint,
            DARK,
        );
        Ok(())
    }

    fn type_card(&mut self) -> Result<(), String> {
        let logical = self.extent.map(|size| size / u32::from(self.scale));
        let x = f64::from(logical[0].saturating_sub(490) / 2);
        let y = f64::from(logical[1].saturating_sub(383) / 2);
        let samples = [(12, 20), (16, 24), (24, 35)];
        self.rect([x, y], [490, 143], DARK);
        self.rect([x, y + 143.0], [490, 143], [0.89, 0.91, 0.91, 1.0]);
        let mut line = y + 12.0;
        for (points, height) in samples {
            self.text(Text {
                extent: [460, height],
                ..Text::at(
                    "Sphinx of black quartz, judge my vow.",
                    [x + 14.0, line],
                    points,
                )
            })?;
            line += f64::from(height);
        }
        self.text(Text {
            extent: [460, 22],
            tint: WHITE,
            ..Text::at(
                "AV To r. · 0123456789 — “…” (€ × −)",
                [x + 14.0, y + 91.0],
                12,
            )
        })?;
        self.pixel_pair(x, y + 117.0, WHITE, false)?;
        self.text(Text {
            extent: [460, 20],
            tint: DARK,
            ..Text::at(
                "AV To r. · 0123456789 — “…” (€ × −)",
                [x + 14.0, y + 153.0],
                12,
            )
        })?;
        self.text(Text {
            extent: [460, 26],
            tint: DARK,
            ..Text::at("Élève français · Grüße aus Köln", [x + 14.0, y + 172.0], 16)
        })?;
        self.text(Text {
            extent: [460, 36],
            tint: DARK,
            ..Text::at("Zażółć gęślą jaźń", [x + 14.0, y + 196.0], 24)
        })?;
        self.text(Text {
            extent: [460, 25],
            tint: DARK,
            ..Text::at(
                "Příliš žluťoučký kůň úpěl ďábelské ódy.",
                [x + 14.0, y + 232.0],
                16,
            )
        })?;
        self.pixel_pair(x, y + 262.0, DARK, false)?;
        self.text(Text {
            extent: [460, 28],
            outlined: true,
            ..Text::at(
                "AV To r. · À Noël, déjà l’hiver.",
                [x + 14.0, y + 303.0],
                16,
            )
        })?;
        self.pixel_pair(x, y + 343.0, WHITE, true)
    }

    fn pixel_pair(
        &mut self,
        x: f64,
        top: f64,
        tint: [f32; 4],
        outlined: bool,
    ) -> Result<(), String> {
        for (text, points, left, top) in [
            ("PIXEL GRID 0123456789", 8, x + 14.0, top + 4.0),
            ("PIXEL 0123", 16, x + 196.0, top),
        ] {
            self.text(Text {
                kind: PIXEL,
                outlined,
                tint,
                extent: [180, 22],
                ..Text::at(text, [left, top], points)
            })?;
        }
        Ok(())
    }

    fn counting_card(&mut self, tick: u64) -> Result<(), String> {
        let x = f64::from(self.extent[0] / u32::from(self.scale)).min(840.0) / 2.0 - 150.0;
        let y = 84.0;
        self.rect([x, y], [300, 106], DARK);
        let count = 97 + tick / 18;
        self.text(Text {
            extent: [265, 35],
            alignment: Alignment::Right,
            ..Text::at(
                &format!("Packets received     {count}"),
                [x + 14.0, y + 10.0],
                24,
            )
        })?;
        let centis = tick.saturating_mul(100) / 60;
        self.text(Text {
            alignment: Alignment::Right,
            extent: [265, 25],
            tabular: true,
            ..Text::at(
                &format!(
                    "{:02}:{:02}.{:02}",
                    centis / 6000,
                    centis / 100 % 60,
                    centis % 100
                ),
                [x + 14.0, y + 46.0],
                16,
            )
        })?;
        let drift = f64::from(u16::try_from(tick % 240).unwrap_or_default()) * 0.15;
        self.text(Text::at(
            "AV To r. · Élan en mouvement",
            [x + 14.0 + drift, y + 77.0],
            12,
        ))
    }

    fn comparison(&mut self, digit: u64) -> Result<(), String> {
        self.rect([0.0, 0.0], [128, 80], [0.0, 0.0, 0.0, 1.0]);
        self.text(Text {
            extent: [106, 22],
            ..Text::at("AV To r. Éž", [12.0, 12.0], 12)
        })?;
        self.text(Text {
            kind: PIXEL,
            extent: [24, 16],
            wrap: true,
            ..Text::at(&format!("AA {digit}B"), [12.0, 40.0], 8)
        })?;
        self.text(Text {
            kind: PIXEL,
            extent: [36, 8],
            elision: Elision::Middle,
            ..Text::at(&format!("ABAB{digit}B"), [12.0, 65.0], 8)
        })
    }
}
