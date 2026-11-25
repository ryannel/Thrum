//! Release profiling of font cooking, committed residency and CPU text layout.

#[cfg(target_os = "macos")]
mod metal {
    use std::hint::black_box;
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use serde_json::{Value, json};
    use thrum_asset::{AssetStore, FontProduct, Profile};
    use thrum_cook::{CookConfig, Cooker, Durability, Quality};
    use thrum_render::runtime::FrameConfig;
    use thrum_ui::{Alignment, Elision, TextOptions, TextRun};
    use v1_game::{GameSetup, InterfaceText};

    pub(super) fn run() {
        let root = PathBuf::from(std::env::args_os().nth(1).expect("new scratch directory"));
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/game-ui/font-source");
        let product = ["type", "pixel"].map(|name| {
            let source_root = root.join(format!("source-{name}"));
            let folder = source_root.join(name);
            std::fs::create_dir_all(&folder).expect("create font source");
            for entry in std::fs::read_dir(source.join(name)).expect("source font") {
                let entry = entry.expect("source file");
                std::fs::copy(entry.path(), folder.join(entry.file_name())).expect("copy source");
            }
            let store = root.join(format!("store-{name}"));
            let (store, cook_ns) = cook(source_root, store);
            let product = read_font(&store, name);
            let [sheet_1, sheet_2, sheet_3] = product.sheet_bytes();
            json!({ "font": name, "cook_ns": cook_ns, "table_bytes": product.table_bytes(),
                "sheet_bytes": [sheet_1, sheet_2, sheet_3], "layout": layout(&product, name) })
        });
        let (store, _) = cook(source, root.join("residency"));
        let gpu = thrum_test_support::open(
            "font costs",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Native,
            &thrum_test_support::Restriction::default(),
        )
        .expect("Mac Metal device");
        let resident = [1, 2, 3].map(|scale| {
            let setup = GameSetup {
                sprite_scale: Some(2),
                sprite_rows: Some(1_000),
                interface_scale: Some(scale),
                interface_text: Some(InterfaceText::Specimen),
                ..Default::default()
            };
            let mut host = pollster::block_on(setup.prepare_with_sheet_source(
                &gpu.device,
                &gpu.queue,
                &gpu.capabilities,
                FrameConfig {
                    render_extent: [160, 96],
                    surface: None,
                },
                store.root().to_path_buf().into(),
            ))
            .expect("prepare font scene");
            let stats = host
                .source()
                .sprite_sheet_stats()
                .expect("residency account");
            thrum_asset_gpu::await_drawn(&mut host, &stats, thrum_asset_gpu::Drawn::Generation(1))
                .expect("submit font scroll");
            let record = stats.lock().expect("residency account");
            assert!(record.errors.is_empty(), "{:?}", record.errors);
            json!({ "scale": scale, "bytes": record.resident_font_bytes, "drawn": record.drawn })
        });
        let output = json!({ "adapter": gpu.capabilities.adapter().name, "products": product, "resident": resident });
        writeln!(std::io::stdout().lock(), "{output}").expect("write cost log");
    }

    fn cook(source: PathBuf, store: PathBuf) -> (AssetStore, u128) {
        let mut cooker = Cooker::open(CookConfig {
            source,
            store,
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("open cooker");
        let report = cooker.cook_all().expect("cook");
        assert!(report.failed.is_empty() && report.cooked > 0, "{report:?}");
        (
            AssetStore::new(cooker.store().root()),
            report.cook_time.as_nanos(),
        )
    }

    fn read_font(store: &AssetStore, name: &str) -> FontProduct {
        let head = store.read_head().expect("HEAD").expect("publication");
        let manifest = store.load_manifest(&head).expect("manifest");
        let entry = manifest.resolve(name, &[Profile::Portable]).expect("font");
        FontProduct::open(store.map(&entry.product).expect("font bytes")).expect("validated font")
    }

    fn layout(font: &FontProduct, name: &str) -> [Value; 3] {
        let points = if name == "type" { 16.0 } else { 8.0 };
        [10, 100, 1_000].map(|glyphs| {
            let text: String = "AB01234567".chars().cycle().take(glyphs).collect();
            let options = TextOptions {
                points,
                scale: 1,
                extent: [32_768, 48],
                alignment: Alignment::Left,
                wrap: false,
                elision: Elision::End,
                tabular_digits: false,
                outlined: false,
            };
            let check = TextRun::new(font, &text, options).expect("layout");
            assert_eq!(check.glyphs().len(), glyphs);
            assert_eq!(check.missing(), 0);
            let samples = std::array::from_fn::<_, 32, _>(|_| {
                let start = Instant::now();
                black_box(
                    TextRun::new(black_box(font), black_box(&text), options).expect("layout"),
                );
                start.elapsed().as_nanos()
            });
            json!({ "glyphs": glyphs, "raw_cpu_ns": samples })
        })
    }
}

#[cfg(target_os = "macos")]
fn main() {
    metal::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}
