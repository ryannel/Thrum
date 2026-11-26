mod font;

use thrum_ui::{
    Alignment, Elision, Font, Image, Node, NodeKind, PaintInputs, Size, Text, TextOptions, Viewport,
};

pub(crate) fn view(extent: [u32; 2]) -> Viewport {
    Viewport {
        extent,
        scale: 1,
        insets: [0.0; 4],
    }
}

pub(crate) fn font() -> [Font<'static>; 1] {
    [Font {
        product: font::pixel(),
        sheet: 1,
    }]
}

pub(crate) fn inputs<'a>(fonts: &'a [Font<'a>]) -> PaintInputs<'a> {
    PaintInputs {
        stamp: None,
        fonts,
        white: Image {
            sheet: 0,
            frame: 0,
            texels: [1, 1],
        },
    }
}

pub(crate) fn fill(id: u32, size: [f64; 2]) -> Node {
    let mut node = Node::new(id, NodeKind::Fill([1.0; 4]));
    node.size = size.map(Size::fixed);
    node
}

pub(crate) fn text(id: u32, value: &str, wrap: bool) -> Node {
    Node::new(
        id,
        NodeKind::Text(Text {
            value: value.into(),
            font: 0,
            options: TextOptions {
                points: 8.0,
                scale: 1,
                extent: [0; 2],
                alignment: Alignment::Left,
                wrap,
                elision: Elision::End,
                tabular_digits: true,
                outlined: false,
            },
            tint: [1.0; 4],
            outline: [0.0; 4],
        }),
    )
}
