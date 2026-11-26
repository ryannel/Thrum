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

use crate::{
    interface_card::Card,
    interface_menu::{MenuFrame, MenuView},
};

const TYPE: &str = "type";
const PIXEL: &str = "pixel";
const STATUS: &str = "status";
const WHITE: [f32; 4] = [1.0; 4];
const DARK: [f32; 4] = [0.015, 0.024, 0.041, 1.0];

/// Text cards of the proving scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceText {
    /// Light, dark and outlined text with European Latin samples.
    Type,
    /// Health, timer and counters over the moving sprite field.
    Status,
    /// The interactive choice and join screen.
    Menu,
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
    insets: [f64; 4],
    density: f64,
    forced_scale: Option<u8>,
    measure_updates: bool,
    held_tick: Option<u64>,
    menu: MenuView,
}

struct Layout {
    stamp: PaintStamp,
    extent: [u32; 2],
    value: u64,
    paint: PaintList,
    tree: Option<(crate::interface_status::Appearance, thrum_ui::Tree)>,
    menu: Option<crate::interface_menu::MenuLayout>,
}

impl FontCard {
    pub(crate) fn attach(
        source: CookedSource,
        mode: InterfaceText,
        scale: Option<u8>,
    ) -> Result<(Self, CookedAssets), FrameError> {
        let mut config = CookedConfig::new(source, Vec::new());
        config.lease_font(TYPE);
        if mode == InterfaceText::Status {
            config.lease_sheet(STATUS);
        } else if mode != InterfaceText::Menu {
            config.lease_font(PIXEL);
        }
        if mode == InterfaceText::Specimen {
            config.upload_budget = thrum_asset_gpu::COPY_UNIT;
        }
        let request = config.font_scale_request();
        if !request.set(u32::from(scale.unwrap_or(1))) {
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
                insets: [0.0; 4],
                density: 1.0,
                forced_scale: scale,
                measure_updates: false,
                held_tick: None,
                menu: MenuView::default(),
            },
            assets,
        ))
    }

    pub(crate) fn config(&self) -> InterfaceConfig {
        let mut config = Card::new(1, None, true).config();
        config.row_capacity = 2048;
        let second = if self.mode == InterfaceText::Status {
            config.sheets = vec![crate::interface_status::panel_sheet()];
            STATUS
        } else {
            PIXEL
        };
        #[cfg_attr(
            target_arch = "wasm32",
            expect(
                clippy::arc_with_non_send_sync,
                reason = "the browser's lease stays on its render thread"
            )
        )]
        let source = Arc::new(self.lease.clone());
        config.cooked = Some(InterfaceSheets {
            names: if self.mode == InterfaceText::Menu {
                vec![TYPE.to_owned()]
            } else {
                [TYPE, second].map(str::to_owned).into()
            },
            source,
        });
        config
    }

    pub(crate) fn set_scale(&self, scale: u32) -> bool {
        self.scale.set(scale)
    }

    #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
    pub(crate) fn output(&mut self, density: f64, insets: [f64; 4]) {
        if matches!(self.mode, InterfaceText::Status | InterfaceText::Menu) {
            self.insets = insets;
            self.density = density;
            if self.forced_scale.is_none() {
                self.scale.set(if density < 1.5 {
                    1
                } else if density < 2.5 {
                    2
                } else {
                    3
                });
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn hold_status_capture(&mut self, status: crate::StatusCapture) {
        self.held_tick = status.tick;
        let scale = f64::from(self.scale.get());
        self.density = status.density.unwrap_or(scale);
        // Capture insets are already in interface points.
        self.insets = status.insets.map(|inset| inset * scale / self.density);
    }

    pub(crate) const fn measure_updates(&mut self) {
        self.measure_updates = true;
    }

    pub(crate) fn status_stats(&self) -> Option<thrum_ui::TreeStats> {
        self.active
            .as_ref()?
            .tree
            .as_ref()
            .map(|(_, tree)| tree.stats())
    }

    #[cfg(target_os = "ios")]
    pub(crate) fn status_bounds(&self) -> Option<serde_json::Value> {
        let stamp = self.lease.committed_font()?.stamp;
        let (_, tree) = self
            .prepared
            .iter()
            .chain(self.active.iter())
            .find(|layout| layout.stamp == stamp)?
            .tree
            .as_ref()?;
        let nodes = (0..=8).chain(
            crate::interface_status::COUNTS
                .into_iter()
                .flat_map(|counter| counter - 2..=counter + 1),
        );
        Some(serde_json::json!(
            nodes
                .map(|id| {
                    (
                        id,
                        tree.bounds(id).map(|bounds| [bounds.origin, bounds.end()]),
                    )
                })
                .collect::<Vec<_>>()
        ))
    }

    pub(crate) const fn interactive(&self) -> bool {
        matches!(self.mode, InterfaceText::Menu)
    }

    #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
    pub(crate) fn set_menu(&mut self, view: MenuView) {
        self.menu = view;
    }

    #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
    pub(crate) fn submitted_menu(&self) -> Option<MenuFrame> {
        if self.mode != InterfaceText::Menu {
            return None;
        }
        let stamp = self.lease.committed_font()?.stamp;
        self.prepared
            .iter()
            .chain(self.active.iter())
            .find(|layout| layout.stamp == stamp)?
            .menu
            .as_ref()
            .map(|menu| menu.frame.clone())
    }

    pub(crate) fn paint(&mut self, extent: [u32; 2], tick: u64) -> [Option<PaintList>; 2] {
        let value = match self.mode {
            InterfaceText::Type | InterfaceText::Menu => 0,
            InterfaceText::Status => self.held_tick.unwrap_or(tick),
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
        let view = (self.density, self.insets, self.measure_updates);
        let menu = (self.mode == InterfaceText::Menu).then(|| self.menu.for_frame(tick));
        let active = committed.as_ref().and_then(|font| {
            Self::layout(
                &mut self.active,
                self.mode,
                font,
                extent,
                value,
                view,
                menu.as_ref(),
            )
            .ok()
        });
        let prepared = if let Some(font) = self.lease.font_candidate() {
            match Self::layout(
                &mut self.prepared,
                self.mode,
                &font,
                extent,
                value,
                view,
                menu.as_ref(),
            ) {
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
        view: (f64, [f64; 4], bool),
        menu: Option<&Arc<crate::interface_menu::View>>,
    ) -> Result<PaintList, String> {
        if mode == InterfaceText::Status {
            return Self::status(cached, fonts, extent, value, view);
        }
        if let Some(menu) = menu {
            return Self::menu(cached, fonts, extent, (view.0, view.1), menu);
        }
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
            InterfaceText::Status | InterfaceText::Menu => {
                unreachable!("tree-based interface already handled")
            }
        }
        let paint = card.builder.finish();
        *cached = Some(Layout {
            stamp: fonts.stamp,
            extent,
            value,
            paint: paint.clone(),
            tree: None,
            menu: None,
        });
        Ok(paint)
    }
    fn menu(
        cached: &mut Option<Layout>,
        fonts: &FontCandidate,
        extent: [u32; 2],
        (density, insets): (f64, [f64; 4]),
        view: &Arc<crate::interface_menu::View>,
    ) -> Result<PaintList, String> {
        let stamp = fonts.stamp;
        let viewport = thrum_ui::Viewport {
            extent,
            insets: insets.map(|inset| inset * density / f64::from(stamp.scale)),
            scale: stamp.scale,
        };
        if let Some(layout) = cached
            && layout.stamp == stamp
            && let Some(menu) = &layout.menu
            && menu.frame.viewport == viewport
            && Arc::ptr_eq(&menu.frame.view, view)
        {
            return Ok(layout.paint.clone());
        }
        let font = fonts.font(TYPE).ok_or("interface menu font is missing")?;
        let retained = cached.as_mut().and_then(|layout| layout.menu.take());
        let (paint, menu) = crate::interface_menu::MenuLayout::paint(
            retained,
            Arc::clone(view),
            font,
            stamp,
            viewport,
        )
        .map_err(|error| error.to_string())?;
        *cached = Some(Layout {
            stamp,
            extent,
            value: 0,
            paint: paint.clone(),
            tree: None,
            menu: Some(menu),
        });
        Ok(paint)
    }
    fn status(
        cached: &mut Option<Layout>,
        fonts: &FontCandidate,
        extent: [u32; 2],
        tick: u64,
        (density, insets, measure_updates): (f64, [f64; 4], bool),
    ) -> Result<PaintList, String> {
        use crate::interface_status::{self, WHITE_TILE};
        let font = fonts.font(TYPE).ok_or("status display font is missing")?;
        let badge = fonts
            .sheet(STATUS)
            .and_then(|sheet| {
                sheet
                    .frames()
                    .enumerate()
                    .find(|(_, (name, _))| *name == "health")
            })
            .ok_or("status display badge is missing")?;
        let (index, (_, badge)) = badge;
        if badge.trim != [0; 2] || badge.pivot.into_iter().any(|value| value.abs() > 0.0) {
            return Err("status display badge must use its top-left pivot and full cell".into());
        }
        let viewport = thrum_ui::Viewport {
            extent,
            insets: insets.map(|inset| inset * density / f64::from(fonts.stamp.scale)),
            scale: fonts.stamp.scale,
        };
        let appearance = interface_status::Appearance::for_output(viewport, density);
        let mut tree = cached
            .as_mut()
            .filter(|layout| layout.stamp == fonts.stamp)
            .and_then(|layout| layout.tree.take())
            .filter(|(cached, _)| *cached == appearance)
            .map(|(_, tree)| tree)
            .map_or_else(
                || {
                    interface_status::tree_with_appearance(
                        thrum_ui::Image {
                            sheet: 2,
                            frame: u32::try_from(index).expect("validated sheet count fits u32"),
                            texels: badge.size,
                        },
                        appearance,
                    )
                    .map_err(|error| error.to_string())
                },
                Ok,
            )?;
        if measure_updates {
            interface_status::update_values(&mut tree, tick, true);
        } else {
            interface_status::update(&mut tree, tick);
        }
        let paint = tree
            .frame(
                viewport,
                thrum_ui::PaintInputs {
                    stamp: Some(fonts.stamp),
                    fonts: &[thrum_ui::Font {
                        product: font,
                        sheet: 1,
                    }],
                    white: WHITE_TILE,
                },
            )
            .map_err(|error| error.to_string())?;
        *cached = Some(Layout {
            stamp: fonts.stamp,
            extent,
            value: tick,
            paint: paint.clone(),
            tree: Some((appearance, tree)),
            menu: None,
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
