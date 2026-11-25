//! Named refusals never make an invalid font table or a partial cook visible.

mod font_support;

use std::mem::offset_of;

use thrum_asset::{
    FontProduct, ProductBytes,
    layout::{ContainerHeader, FontGlyph, FontSize, Section},
};
use thrum_test_support::Scratch;

#[test]
fn malformed_product_and_unusable_edits_are_named_and_last_good_is_retained() {
    let scratch = Scratch::new("font-hostile");
    let folder = scratch.join("source/type");
    font_support::copy(&font_support::fixture("type"), &folder);
    let description = folder.join("font.json");
    let small = |points, characters: &str| {
        format!(
            r#"{{"kind":"outline","file":"Roboto-Regular.ttf","sizes":[{{"points":{points}}}],"characters":"{characters}"}}"#
        )
    };
    std::fs::write(&description, small(12, "AV")).expect("font source");
    let mut cooker = font_support::cooker(scratch.join("source"), scratch.join("store"));
    assert_eq!(cooker.cook_all().expect("first cook").failed, []);
    let good = font_support::product(cooker.store(), "type");
    assert!(FontProduct::open(good.clone()).is_ok());
    let head = cooker.store().read_head().expect("first head");
    let section = |index| size_of::<ContainerHeader>() + index * size_of::<Section>();
    let payload = |index| {
        let offset = section(index) + offset_of!(Section, offset);
        usize::try_from(u64::from_le_bytes(
            good[offset..offset + 8].try_into().expect("offset"),
        ))
        .expect("bounded")
    };
    for (offset, word, reason) in [
        (
            payload(0) + offset_of!(FontSize, ascent),
            f32::NAN.to_bits(),
            "font metric is not finite",
        ),
        (
            payload(0) + size_of::<FontSize>() + offset_of!(FontSize, scale),
            1,
            "duplicate size and scale",
        ),
        (
            payload(1) + offset_of!(FontGlyph, frame),
            u32::MAX,
            "glyph names a missing sheet frame",
        ),
        (
            section(4),
            u32::from_le_bytes(*b"MISS"),
            "sheet missing for listed scale",
        ),
    ] {
        let mut bad = good.to_vec();
        bad[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
        assert!(
            FontProduct::open(ProductBytes::copy_from(&bad))
                .expect_err(reason)
                .to_string()
                .contains(reason)
        );
    }
    assert!(
        FontProduct::open(ProductBytes::copy_from(&good[..good.len() - 1]))
            .expect_err("truncated font")
            .to_string()
            .contains("font table truncated")
    );
    for (source, reason) in [
        (small(0, "AV"), "font size must be positive"),
        (small(12, "A🦉"), "font lacks listed character"),
        (small(10, "AV"), "font file does not parse"),
    ] {
        if reason == "font file does not parse" {
            std::fs::write(folder.join("Roboto-Regular.ttf"), b"bad").expect("invalid source");
        }
        std::fs::write(&description, source).expect("edit description");
        let report = cooker.cook_all().expect("failed edit is reported");
        assert!(
            report
                .failed
                .iter()
                .any(|(name, error)| name == "type" && error.contains(reason)),
            "{reason}: {report:?}"
        );
        assert_eq!(cooker.store().read_head().expect("HEAD after edit"), head);
    }
}
