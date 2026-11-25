//! Original panel card and paint lists for the interface-stage proofs.

use thrum_render::sprite::{EmbeddedSheet, NamedSheet, SpriteRect};
use thrum_ui::{InterfaceConfig, PaintList, PaintListBuilder, SpriteRow};

const MARKS: [[u8; 4]; 6] = [
    [255, 134, 61, 255],
    [105, 247, 185, 255],
    [223, 229, 255, 255],
    [48, 70, 192, 255],
    [185, 70, 237, 255],
    [6, 117, 110, 255],
];

pub(crate) struct Card {
    scale: u8,
    load_rows: Option<u32>,
    over_sprites: bool,
    paint: Option<PaintList>,
    reuse_next: bool,
    green: bool,
}

impl Card {
    pub(crate) fn new(scale: u8, load_rows: Option<u32>, over_sprites: bool) -> Self {
        Self {
            scale,
            load_rows,
            over_sprites,
            paint: None,
            reuse_next: false,
            green: false,
        }
    }

    pub(crate) fn config(&self) -> InterfaceConfig {
        let mut rgba = Vec::from([255; 4]);
        rgba.extend(MARKS.into_iter().flatten());
        let mut sheet = NamedSheet::pixel(
            "panel card",
            EmbeddedSheet {
                extent: [7, 1],
                rgba: rgba.into(),
                frames: (0..7)
                    .map(|x| SpriteRect {
                        origin: [x, 0],
                        size: [1, 1],
                    })
                    .collect::<Vec<_>>()
                    .into(),
                texels_per_unit: 1.0,
            },
        );
        sheet.tiles = (0..7).collect();
        InterfaceConfig {
            sheets: vec![sheet],
            row_capacity: self.load_rows.unwrap_or(32).max(1),
        }
    }

    pub(crate) fn paint(&mut self, extent: [u32; 2]) -> PaintList {
        if let Some(list) = &self.paint
            && list.extent == extent
            && (self.load_rows.unwrap_or_default() == 0 || self.reuse_next)
        {
            self.reuse_next = false;
            return list.clone();
        }
        self.reuse_next = true;
        let mut builder = PaintListBuilder::new(extent);
        match self.load_rows {
            Some(rows) => {
                self.green = !self.green;
                let tint = if self.green {
                    [0.2, 0.8, 0.5, 1.0]
                } else {
                    [1.0; 4]
                };
                for index in 0..rows {
                    let x = index % 100 * 5;
                    let y = index / 100 * 5;
                    let mut row = self.rect(0, [x, y], [4, 4]);
                    row.tint = tint;
                    builder.push(row);
                }
            }
            None => self.panel(extent, &mut builder),
        }
        let list = builder.finish();
        self.paint = Some(list.clone());
        list
    }

    fn rect(&self, frame: u32, origin: [u32; 2], size: [u32; 2]) -> SpriteRow {
        let origin = origin.map(|pixel| f64::from(pixel) * f64::from(self.scale));
        let mut row = SpriteRow::new(frame, origin);
        row.scale = size.map(|pixel| pixel as f32 * f32::from(self.scale));
        row
    }

    fn panel(&self, extent: [u32; 2], builder: &mut PaintListBuilder) {
        let scale = u32::from(self.scale);
        let [left, top] = if self.over_sprites {
            extent.map(|side| side / (4 * scale))
        } else {
            [0; 2]
        };
        let rect = |frame, [x, y]: [u32; 2], size| self.rect(frame, [left + x, top + y], size);
        let mut scrim = rect(0, [8, 8], [112, 58]);
        scrim.tint = [0.0, 0.0, 0.0, 0.5];
        builder.push(scrim);
        for (frame, origin, size) in [
            (1, [8, 8], [9, 4]),
            (2, [111, 8], [9, 6]),
            (4, [8, 59], [4, 7]),
            (5, [114, 62], [6, 4]),
            (3, [22, 19], [40, 2]),
            (2, [22, 28], [54, 6]),
            (6, [76, 28], [24, 6]),
        ] {
            builder.push(rect(frame, origin, size));
        }
        for (column, frame) in (1..=6).enumerate() {
            let column = u32::try_from(column).expect("six panel swatches");
            builder.push(rect(frame, [22 + column * 13, 45], [11, 8]));
        }
        let anchor = extent.map(|side| side.saturating_sub(10 * scale));
        let mut mark = self.rect(3, [0; 2], [4, 4]);
        mark.position = anchor.map(f64::from);
        builder.push(mark);
    }
}
