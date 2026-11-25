//! Hostile table and array data refused by the runtime sheet reader.

mod sheet_support;
use sheet_support as setup;

use std::mem::offset_of;

use thrum_asset::layout::{ContainerHeader, Section, SheetFrame, SheetMeta};
use thrum_asset::{ProductBytes, Profile, SheetProduct};
use thrum_test_support::Scratch;

#[test]
fn corrupt_tables_pages_rectangles_names_pivots_and_density_are_refused() {
    let scratch = Scratch::new("sheet-hostile");
    let folder = setup::root(scratch.path());
    std::fs::write(
        folder.join("sheet.json"),
        r#"{"kind":"pixel","density":1,"grids":{"walk.png":[2,2]}}"#,
    )
    .expect("description");
    image::RgbaImage::from_pixel(4, 2, image::Rgba([255; 4]))
        .save(folder.join("walk.png"))
        .expect("image");
    let mut cooker = setup::cook(scratch.path(), "store");
    assert_eq!(cooker.cook_all().expect("cook").failed, []);
    let good = setup::sheet_bytes(cooker.store(), Profile::Portable);
    assert!(SheetProduct::open(good.clone()).is_ok());
    let section = |index| size_of::<ContainerHeader>() + index * size_of::<Section>();
    let payload = |index| {
        let offset = section(index) + offset_of!(Section, offset);
        usize::try_from(u64::from_le_bytes(
            good[offset..offset + 8].try_into().expect("section offset"),
        ))
        .expect("bounded offset")
    };
    let meta = payload(0);
    let frame = payload(1);
    let first_name = offset_of!(SheetFrame, name_offset);
    let second = frame + size_of::<SheetFrame>();
    for (offset, word, reason) in [
        (
            section(1) + offset_of!(Section, count),
            1,
            "sheet frame table length",
        ),
        (
            frame + offset_of!(SheetFrame, origin),
            u32::MAX,
            "sheet frame rectangle",
        ),
        (meta + offset_of!(SheetMeta, pages), 99, "sheet page count"),
        (second + first_name, 0, "duplicate sheet frame name"),
        (
            frame + offset_of!(SheetFrame, pivot),
            f32::NAN.to_bits(),
            "sheet frame pivot",
        ),
        (
            meta + offset_of!(SheetMeta, density),
            f32::INFINITY.to_bits(),
            "sheet density",
        ),
    ] {
        let mut bad = good.to_vec();
        bad[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
        let error = SheetProduct::open(ProductBytes::copy_from(&bad)).expect_err(reason);
        assert!(error.to_string().contains(reason), "{error}");
    }
}
