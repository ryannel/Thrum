use std::time::Duration;

use thrum_ui::{
    Anchor, Button, Contact, Event, Image, Intent, Node, NodeId, NodeKind, PaintInputs, Scope,
    Size, Tree, Viewport,
};

pub(crate) const NOW: Duration = Duration::from_secs(1);
pub(crate) const SCOPE: NodeId = 1;

pub(crate) fn viewport() -> Viewport {
    Viewport {
        extent: [1200, 900],
        insets: [0.0; 4],
        scale: 1,
    }
}

pub(crate) fn layout(tree: &mut Tree, viewport: Viewport) {
    tree.prepare_layout(
        viewport,
        PaintInputs {
            stamp: None,
            fonts: &[],
            white: Image {
                sheet: 0,
                frame: 0,
                texels: [1, 1],
            },
        },
    )
    .expect("valid control layout");
}

pub(crate) fn tree(children: Vec<Node>) -> Tree {
    let mut scope = Node::new(SCOPE, NodeKind::Scope(Scope::default()));
    scope.size = [Size::fixed(800.0); 2];
    scope.children = children;
    let mut tree = Tree::new(scope).expect("valid control declarations");
    layout(&mut tree, viewport());
    tree
}

pub(crate) fn button(id: NodeId, origin: [f64; 2], size: [f64; 2]) -> Node {
    let mut button = Node::new(id, NodeKind::Button(Button::default()));
    button.anchor = origin.map(Anchor::Point);
    button.size = size.map(Size::fixed);
    button
}

pub(crate) fn press(
    contact: Contact<u8>,
    position: Option<[f64; 2]>,
    intent: Option<Intent>,
) -> Event<u8> {
    Event::Press {
        contact,
        position,
        intent,
    }
}
