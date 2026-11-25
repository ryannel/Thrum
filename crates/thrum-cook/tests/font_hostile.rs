//! Named refusals never make an invalid font table or a partial cook visible.

mod font_support;

use std::mem::offset_of;

use thrum_asset::{
    FontProduct, ProductBytes, Profile,
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

#[test]
fn font_addition_and_edits_keep_sibling_bindings_and_the_grid_stays_private() {
    let scratch = Scratch::new("font-siblings");
    let folder = scratch.join("source/assets");
    std::fs::create_dir_all(&folder).expect("assets folder");
    thrum_cook::demo::write_grid_gltf(&folder, "terrain", 4, 0.2).expect("sibling glTF and buffer");
    thrum_cook::demo::write_checker_png(&folder.join("checker.png"), 8, [20, 40, 80])
        .expect("sibling PNG");
    let mut cooker = font_support::cooker(scratch.join("source"), scratch.join("store"));
    assert_eq!(
        cooker.cook_all().expect("cook standalone assets").failed,
        []
    );
    let store = cooker.store().clone();
    let manifest = || {
        store
            .load_manifest(&store.read_head().unwrap().unwrap())
            .unwrap()
    };
    let before = manifest();
    let mesh = before
        .resolve("assets/terrain", &[Profile::Portable])
        .unwrap();
    let textures: Vec<_> = Profile::ALL
        .iter()
        .map(|&profile| {
            (
                profile,
                before
                    .resolve("assets/checker", &[profile])
                    .unwrap()
                    .product,
            )
        })
        .collect();
    let check_siblings = || {
        let current = manifest();
        assert_eq!(
            current
                .resolve("assets/terrain", &[Profile::Portable])
                .unwrap()
                .product,
            mesh.product
        );
        for &(profile, hash) in &textures {
            assert_eq!(
                current
                    .resolve("assets/checker", &[profile])
                    .unwrap()
                    .product,
                hash
            );
            assert!(current.resolve("assets/grid", &[profile]).is_none());
        }
        current
            .resolve("assets", &[Profile::Portable])
            .unwrap()
            .product
    };
    font_support::copy(&font_support::fixture("pixel"), &folder);
    assert_eq!(cooker.cook_all().expect("add the font").failed, []);
    let original = check_siblings();
    let description = folder.join("font.json");
    let edit = std::fs::read_to_string(&description)
        .unwrap()
        .replace("\"advance\": 8", "\"advance\": 9");
    std::fs::write(&description, edit).unwrap();
    assert_eq!(cooker.cook_all().expect("edit the font").failed, []);
    let revised = check_siblings();
    assert_ne!(revised, original);
    std::fs::write(&description, b"{bad json").unwrap();
    assert_eq!(cooker.cook_all().expect("refuse bad edit").failed.len(), 1);
    assert_eq!(check_siblings(), revised);
    println!("font add, edit and refusal retain the sibling mesh and every PNG profile");
}
