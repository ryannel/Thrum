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

#[test]
fn refused_outline_size_allows_a_new_sibling_texture_after_reopen() {
    let scratch = Scratch::new("font-outline-sibling");
    let source = scratch.join("source");
    let store = scratch.join("store");
    let folder = source.join("hud");
    font_support::copy(&font_support::fixture("type"), &folder);
    let description = folder.join("font.json");
    let outline = |points| {
        format!(
            r#"{{"kind":"outline","file":"Roboto-Regular.ttf","sizes":[{{"points":{points}}}],"characters":"AV"}}"#
        )
    };
    std::fs::write(&description, outline(12)).unwrap();
    let cook = || {
        let mut cooker = font_support::cooker(source.clone(), store.clone());
        cooker.cook_all().unwrap()
    };
    let head = cook().published.unwrap();
    let reader = thrum_asset::AssetStore::new(store.clone());
    let font_hash = |manifest: &thrum_asset::Manifest| {
        manifest
            .resolve("hud", &[Profile::Portable])
            .unwrap()
            .product
    };
    let original = font_hash(&reader.load_manifest(&head).unwrap());
    std::fs::write(&description, outline(0)).unwrap();
    let refused = |report: &thrum_cook::CookReport| {
        assert!(report.failed.iter().any(|(name, reason)| {
            name == "hud" && reason.contains("font size must be positive")
        }));
    };
    let report = cook();
    refused(&report);
    assert!(report.published.is_none());

    thrum_cook::demo::write_checker_png(&folder.join("badge.png"), 4, [80, 40, 20]).unwrap();
    let report = cook();
    refused(&report);
    let next = report
        .published
        .expect("the new badge publishes during refusal");
    assert_eq!(reader.read_head().unwrap(), Some(next));
    let manifest = reader.load_manifest(&next).unwrap();
    assert_eq!(font_hash(&manifest), original);
    for profile in Profile::ALL {
        assert!(manifest.resolve("hud/badge", &[profile]).is_some());
    }
    std::fs::write(&description, outline(12)).unwrap();
    let report = cook();
    assert!(report.failed.is_empty());
    assert!(
        report.published.is_none(),
        "both products are already published"
    );
}

#[test]
fn refused_grid_rename_preserves_the_last_manifest_even_after_reopen() {
    let scratch = Scratch::new("font-grid-rename");
    let source = scratch.join("source");
    let store = scratch.join("store");
    let folder = source.join("hud");
    font_support::copy(&font_support::fixture("pixel"), &folder);
    let description = folder.join("font.json");
    let original = std::fs::read_to_string(&description).expect("font description");
    let renamed = original.replace("grid.png", "icon.png");
    let icon = folder.join("icon.png");
    thrum_cook::demo::write_checker_png(&icon, 4, [20, 40, 80]).expect("standalone icon");
    let badge = folder.join("badge.png");
    thrum_cook::demo::write_checker_png(&badge, 4, [80, 40, 20]).expect("standalone badge");
    let mut cooker = font_support::cooker(source.clone(), store.clone());
    let head = cooker
        .cook_all()
        .expect("first cook")
        .published
        .expect("initial head");
    let original_manifest = cooker
        .store()
        .load_manifest(&head)
        .expect("initial manifest");
    for profile in Profile::ALL {
        assert!(original_manifest.resolve("hud/icon", &[profile]).is_some());
        assert!(original_manifest.resolve("hud/grid", &[profile]).is_none());
    }

    std::fs::write(&description, &renamed).expect("try the icon as a grid");
    for reopen in [false, true] {
        if reopen {
            drop(cooker);
            cooker = font_support::cooker(source.clone(), store.clone());
        }
        let report = cooker.cook_all().expect("report refused font");
        assert!(report.failed.iter().any(|(name, reason)| {
            name == "hud" && reason.contains("pixel font cells must divide the image")
        }));
        assert_eq!(
            report.published, None,
            "a refused edit must keep every binding"
        );
        assert_eq!(
            cooker.store().read_head().expect("HEAD after refusal"),
            Some(head)
        );
    }

    thrum_cook::demo::write_checker_png(&icon, 4, [20, 80, 40]).expect("still an invalid grid");
    thrum_cook::demo::write_checker_png(&badge, 4, [80, 20, 40]).expect("independent badge edit");
    thrum_cook::demo::write_checker_png(&folder.join("new.png"), 4, [40, 80, 20])
        .expect("independent new texture");
    drop(cooker);
    let mut cooker = font_support::cooker(source, store);
    let badge_head = cooker
        .cook_all()
        .expect("independent edit can still publish")
        .published
        .expect("badge publication");
    let badge_manifest = cooker
        .store()
        .load_manifest(&badge_head)
        .expect("badge manifest");
    for profile in Profile::ALL {
        let product = |manifest: &thrum_asset::Manifest, name| {
            manifest
                .resolve(name, &[profile])
                .expect("published binding")
                .product
        };
        assert_eq!(
            product(&badge_manifest, "hud/icon"),
            product(&original_manifest, "hud/icon")
        );
        assert_ne!(
            product(&badge_manifest, "hud/badge"),
            product(&original_manifest, "hud/badge")
        );
        assert!(badge_manifest.resolve("hud/grid", &[profile]).is_none());
        assert!(badge_manifest.resolve("hud/new", &[profile]).is_some());
    }

    let mut pixels = image::open(folder.join("grid.png"))
        .expect("old grid")
        .into_rgba8();
    pixels.get_pixel_mut(0, 0).0[3] ^= 255;
    pixels
        .save(&icon)
        .expect("replace icon with a valid edited grid");
    let new_head = cooker
        .cook_all()
        .expect("valid edit")
        .published
        .expect("new head");
    assert_eq!(new_head.generation, badge_head.generation + 1);
    let current = cooker
        .store()
        .load_manifest(&new_head)
        .expect("new manifest");
    for profile in Profile::ALL {
        assert!(current.resolve("hud/icon", &[profile]).is_none());
        assert!(current.resolve("hud/grid", &[profile]).is_some());
    }
    assert_ne!(
        current
            .resolve("hud", &[Profile::Portable])
            .unwrap()
            .product,
        original_manifest
            .resolve("hud", &[Profile::Portable])
            .unwrap()
            .product
    );
    println!(
        "refusal kept HEAD and all icon profiles across reopen; an independent badge edit published; a valid rename published the font and binding changes together"
    );
}
