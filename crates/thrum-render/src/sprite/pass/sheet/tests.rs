use super::{EmbeddedSheet, GUTTER, MIP_LEVELS, NamedSheet, upload};
use crate::{
    capabilities::{DeviceProfile, DeviceRestriction},
    sprite::SpriteRect,
};
use thrum_fixtures::sprite_edges;

#[test]
fn edge_witness_has_four_uploaded_neighbours_at_every_mip() {
    let Some(gpu) = crate::test_device::open(
        "sprite edge atlas witness",
        Some(wgpu::Backend::Metal),
        DeviceProfile::Native,
        &DeviceRestriction::default(),
    ) else {
        return;
    };
    let (rgba, origins) = sprite_edges::art();
    let sheet = NamedSheet::pixel(
        "edge witness",
        EmbeddedSheet {
            extent: sprite_edges::EXTENT,
            rgba: rgba.into(),
            frames: origins
                .map(|origin| SpriteRect {
                    origin,
                    size: sprite_edges::FRAME_SIZE,
                })
                .into(),
            texels_per_unit: 1.0,
        },
    );
    let mut table = Vec::new();
    let (atlas, _) =
        upload(&gpu.device, &gpu.queue, &sheet, &mut table).expect("edge atlas uploads");
    let witness = table[5];
    assert_eq!(atlas.mip_level_count(), MIP_LEVELS);
    let neighbours = [4, 6, 1, 9].map(|index| table[index]);
    for index in [4, 6, 1, 9] {
        let [x, y] = sheet.sheet.frames[index].origin;
        assert_eq!(
            sheet.sheet.rgba[((y * sheet.sheet.extent[0] + x) * 4) as usize],
            255
        );
    }
    for level in 0..atlas.mip_level_count() {
        let dimensions = [atlas.width() >> level, atlas.height() >> level];
        let [x, y] = witness.origin.map(|side| side >> level);
        let margin = GUTTER >> level;
        let stride = (witness.size[0] >> level) + 2 * margin;
        let points = [
            [x - stride, y],
            [x + stride, y],
            [x, y - stride],
            [x, y + stride],
        ];
        for (neighbour, expected) in neighbours.iter().zip(points) {
            assert_eq!(neighbour.page, witness.page);
            assert_eq!(neighbour.size, witness.size);
            assert_eq!(neighbour.origin.map(|side| side >> level), expected);
            assert!(
                expected
                    .into_iter()
                    .zip(dimensions)
                    .all(|(start, bound)| { start >= margin && start + stride - margin <= bound })
            );
        }
        println!(
            "mip {level}: page {}, cyan origin [{x}, {y}], neighbour origins {points:?}, gutter {margin}",
            witness.page
        );
    }
}
