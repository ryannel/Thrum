//! Companion to docs/GAME-INTERFACE.md, using the same public APIs as a game.

use std::{
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use thrum_asset::{AssetStore, FontProduct, Profile};
use thrum_asset_gpu::{CookedAssets, CookedConfig};
use thrum_render::sprite::{EmbeddedSheet, NamedSheet, SpriteRect};
use thrum_ui::{
    Alignment, Anchor, Button, Contact, Elision, Event, Font, Image, InterfaceConfig,
    InterfaceSheets, Node, NodeKind, PaintInputs, PaintStamp, Routed, Router, Scope, Size, Text,
    TextOptions, TextSource, Tree, Viewport,
};

const STATUS: u32 = 1;
const CHOICE: u32 = 4;
const ACCEPT: u32 = 6;
const DROP: u32 = 8;
const FONT: &str = "type";
const WHITE: Image = Image {
    sheet: 0,
    frame: 0,
    texels: [1; 2],
};
const SCOPE: Scope = Scope {
    exclusive: true,
    initial: Some(ACCEPT),
};

fn interface_config(assets: &CookedAssets) -> InterfaceConfig {
    let mut white = NamedSheet::pixel(
        "interface white",
        EmbeddedSheet {
            extent: [1; 2],
            rgba: [255; 4].into(),
            frames: [SpriteRect {
                origin: [0; 2],
                size: [1; 2],
            }]
            .into(),
            texels_per_unit: 1.0,
        },
    );
    white.tiles = vec![0];
    InterfaceConfig {
        sheets: vec![white],
        cooked: Some(InterfaceSheets {
            names: vec![FONT.into()],
            source: Arc::new(assets.lease()),
        }),
        row_capacity: 128,
    }
}

fn label(id: u32, value: &str) -> Node {
    Node::new(
        id,
        NodeKind::Text(Text {
            value: value.into(),
            font: 0,
            options: TextOptions {
                points: 12.0,
                scale: 1, // The tree sets each run's scale from its viewport.
                extent: [0; 2],
                alignment: Alignment::Left,
                wrap: false,
                elision: Elision::End,
                tabular_digits: true,
                outlined: false,
            },
            tint: [1.0; 4],
            outline: [0.0; 4],
        }),
    )
}

fn game_tree() -> Result<Tree, thrum_ui::TreeError> {
    let mut root = Node::new(0, NodeKind::Overlay);
    let mut status = Node::new(STATUS, NodeKind::Overlay);
    status.size = [Size::fixed(134.0), Size::fixed(42.0)];
    status.anchor = [Anchor::Point(8.0); 2];
    status.padding = [8.0; 4];
    let mut bar = Node::new(
        3,
        NodeKind::Bar {
            fraction: 1.0,
            track: [0.1, 0.1, 0.15, 1.0],
            fill: [0.2, 0.8, 0.4, 1.0],
        },
    );
    bar.size = [Size::fixed(118.0), Size::fixed(6.0)];
    bar.anchor[1] = Anchor::End;
    let mut background = Node::new(9, NodeKind::Fill([0.02, 0.04, 0.07, 1.0]));
    background.anchor = [Anchor::Centre; 2];
    background.size = status.size;
    status.children = vec![background, label(2, "TIME 00:17"), bar];

    let mut scope = Node::new(CHOICE, NodeKind::Scope(SCOPE));
    scope.size = [Size::fixed(150.0), Size::fixed(70.0)];
    scope.anchor = [Anchor::Centre; 2];
    scope.shown = false;
    let mut card = Node::new(ACCEPT, NodeKind::Button(Button::default()));
    card.size = [Size::fixed(120.0), Size::fixed(44.0)];
    card.anchor = [Anchor::Centre; 2];
    card.padding = [12.0; 4];
    card.children = vec![label(7, "Take shield")];
    let mut background = Node::new(5, NodeKind::Fill([0.1, 0.12, 0.14, 1.0]));
    background.size = scope.size;
    scope.children = vec![background, card, Node::new(DROP, NodeKind::DropTarget(128))];
    root.children = vec![status, scope];
    Tree::new(root)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("supply the cooked store")?,
    );
    let density = 2.0;
    let scale = if density < 1.5 {
        1
    } else if density < 2.5 {
        2
    } else {
        3
    };
    let mut config = CookedConfig::new(path.clone(), Vec::new());
    config.lease_font(FONT);
    assert!(config.font_scale_request().set(scale));
    let assets = CookedAssets::new(config).map_err(io::Error::other)?;
    let interface = interface_config(&assets);
    assert_eq!(interface.sheets.len(), 1);

    // The layout exercise reads the same cooked table directly. In the host,
    // obtain it from the committed or candidate ProductLease publication.
    let store = AssetStore::new(path);
    let head = store.read_head()?.ok_or("cook the font first")?;
    let manifest = store.load_manifest(&head)?;
    let entry = manifest
        .resolve(FONT, &[Profile::Portable])
        .ok_or("missing font")?;
    let font = FontProduct::open(store.map(&entry.product)?)?;
    let viewport = Viewport {
        extent: [600, 390],
        scale,
        insets: [5.0, 10.0, 5.0, 12.0],
    };
    let fonts = [Font {
        product: &font,
        sheet: 1,
    }];
    let inputs = PaintInputs {
        stamp: Some(PaintStamp { head, scale }),
        fonts: &fonts,
        white: WHITE,
    };
    let mut tree = game_tree()?;
    tree.set_fraction(3, 0.7);
    let status = tree.frame(viewport, inputs)?;
    assert!(tree.bounds(STATUS).is_some());

    let now = Duration::ZERO;
    let mut router = Router::<u8>::default();
    assert_eq!(router.open(CHOICE, SCOPE, now), Some(Routed::Interrupted));
    tree.set_shown(CHOICE, true);
    let choice = tree.frame(viewport, inputs)?;
    router.submitted(tree.input_layout()); // In the host, do this after submission.
    assert_eq!(router.focus(), Some(ACCEPT));

    let ticket = String::from("coast/AB+42");
    assert_eq!(
        router.route(Event::Text(ticket.clone(), TextSource::Link), now),
        Some(Routed::Text(DROP, ticket, TextSource::Link)),
    );
    let point = tree
        .bounds(ACCEPT)
        .expect("visible choice")
        .origin
        .map(|axis| axis + 16.0);
    let contact = Contact::Touch(1);
    assert_eq!(
        router.route(
            Event::Press {
                contact,
                position: Some(point),
                intent: None
            },
            now
        ),
        None,
    );
    assert_eq!(
        router.route(Event::Release(contact, Some(point)), now),
        Some(Routed::Activated(ACCEPT))
    );
    let _resume = router.close(CHOICE);
    tree.set_shown(CHOICE, false);
    let _closed = tree.frame(viewport, inputs)?;
    router.submitted(tree.input_layout());
    writeln!(
        io::stdout().lock(),
        "At scale {scale}, status drew {} rows and the choice drew {}. The ticket and activation both arrived.",
        status.rows.len(),
        choice.rows.len()
    )?;
    Ok(())
}
