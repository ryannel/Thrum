use std::sync::Arc;

use thrum_ui::{
    Alignment, Anchor, Axis, Button, ButtonState, Elision, Font, Node, NodeId, NodeKind,
    PaintInputs, PaintList, PaintStamp, Size, SizeRule, Text, TextOptions, Tree, TreeError,
    Viewport,
};

#[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
use thrum_ui::{InputLayout, TreeStats};

use super::{CARDS, CHOICE, COPY, HOST_TICKET, JOIN, SUBMIT, Screen, TICKET_DROP, View, policy};

const FILL: Size = Size::new(SizeRule::Fraction(1.0));
const SHARE: Size = Size::new(SizeRule::Share(1.0));
const CREAM: [f32; 4] = [0.92, 0.94, 0.87, 1.0];
const INK: [f32; 4] = [0.014, 0.029, 0.044, 1.0];
const MUTED: [f32; 4] = [0.43, 0.59, 0.62, 1.0];
const GOLD: [f32; 4] = [0.90, 0.60, 0.25, 1.0];
const TEAL: [f32; 4] = [0.30, 0.76, 0.67, 1.0];

#[derive(Clone)]
pub(crate) struct MenuFrame {
    pub(crate) view: Arc<View>,
    #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
    pub(crate) input: Arc<InputLayout>,
    pub(crate) viewport: Viewport,
    #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
    pub(crate) work: (u64, TreeStats),
}

pub(crate) struct MenuLayout {
    pub(crate) frame: MenuFrame,
    tree: Tree,
    constructions: u64,
}

impl MenuLayout {
    pub(crate) fn paint(
        retained: Option<Self>,
        view: Arc<View>,
        font: &thrum_asset::FontProduct,
        stamp: PaintStamp,
        viewport: Viewport,
    ) -> Result<(PaintList, Self), TreeError> {
        let (mut tree, constructions, previous) = match retained {
            Some(layout)
                if layout.frame.viewport == viewport
                    && same_structure(&layout.frame.view, &view) =>
            {
                (layout.tree, layout.constructions, Some(layout.frame))
            }
            previous => {
                let count = previous.map_or(0, |layout| layout.constructions) + 1;
                (build_tree(&view, viewport)?, count, None)
            }
        };
        if let Some(previous) = &previous {
            edit_style(&mut tree, &previous.view, &view);
        }
        let paint = tree.frame(
            viewport,
            PaintInputs {
                stamp: Some(stamp),
                fonts: &[Font {
                    product: font,
                    sheet: 1,
                }],
                white: crate::interface_status::WHITE_TILE,
            },
        )?;
        #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
        let input = previous
            .filter(|previous| previous.work.1.layouts == tree.stats().layouts)
            .map_or_else(|| Arc::new(tree.input_layout()), |previous| previous.input);
        let frame = MenuFrame {
            view,
            #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
            input,
            viewport,
            #[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
            work: (constructions, tree.stats()),
        };
        Ok((
            paint,
            Self {
                frame,
                tree,
                constructions,
            },
        ))
    }
}

fn same_structure(previous: &View, next: &View) -> bool {
    previous.screen == next.screen
        && match next.screen {
            Screen::Choice => previous.disabled == next.disabled,
            Screen::Play => true,
            _ => previous.ticket == next.ticket,
        }
}

fn edit_style(tree: &mut Tree, previous: &View, view: &View) {
    match view.screen {
        Screen::Play if previous.edit != view.edit => tree.set_text(91, play_line(view.edit)),
        Screen::Choice => {
            if previous.disarmed != view.disarmed {
                tree.set_text(106, choice_prompt(view.disarmed));
            }
            for (index, id) in CARDS.into_iter().enumerate() {
                let old = card_style(previous, index);
                let new = card_style(view, index);
                if old != new || previous.disarmed != view.disarmed {
                    let (background, edge, heading, detail) = card_tints(new, view.disarmed);
                    for (offset, tint) in [(1, edge), (3, background), (5, heading), (6, detail)] {
                        tree.set_tint(id + offset, tint);
                    }
                }
            }
        }
        Screen::Hosting | Screen::Joining if previous.join_button != view.join_button => {
            let id = if view.screen == Screen::Hosting {
                COPY
            } else {
                SUBMIT
            };
            tree.set_tint(id + 1, join_tint(view.join_button));
        }
        _ => {}
    }
}

fn build_tree(view: &View, viewport: Viewport) -> Result<Tree, TreeError> {
    let mut root = Node::new(99, NodeKind::Overlay);
    root.size = [FILL; 2];
    if view.screen == Screen::Play {
        root.children.push(play_prompt(view.edit));
    } else {
        let mut scrim = fill(98, [0.007, 0.014, 0.022, 0.55]);
        scrim.full_bleed = true;
        root.children.push(scrim);
        root.children.push(if view.screen == Screen::Choice {
            choice(view, viewport)
        } else {
            join(view, viewport)
        });
    }
    Tree::new(root)
}

fn play_line(edited: bool) -> &'static str {
    if edited {
        "Field display updated"
    } else if cfg!(target_os = "macos") {
        "M  Choice     J  Join     H  Host"
    } else if cfg!(target_os = "ios") {
        "Choose an upgrade"
    } else {
        "M  Choice     J  Join"
    }
}

fn play_prompt(edited: bool) -> Node {
    let mut panel = fill(90, INK);
    panel.size = [Size::fixed(250.0), Size::fixed(36.0)];
    panel.anchor = [Anchor::Centre, Anchor::Point(8.0)];
    let mut prompt = label(91, play_line(edited), 12.0, CREAM);
    prompt.anchor = [Anchor::Centre; 2];
    #[cfg(target_os = "ios")]
    let mut group = Node::new(super::OPEN_CHOICE, NodeKind::Button(Button::default()));
    #[cfg(not(target_os = "ios"))]
    let mut group = Node::new(92, NodeKind::Overlay);
    group.size = panel.size;
    group.anchor = panel.anchor;
    #[cfg(target_os = "ios")]
    {
        group.size[1] = Size::fixed(44.0);
        panel.size[1] = FILL;
    }
    panel.anchor = [Anchor::Start; 2];
    group.children = vec![panel, prompt];
    group
}

fn scope(id: NodeId, width: f64, height: f64) -> Node {
    let mut panel = Node::new(id, NodeKind::Scope(policy(id)));
    panel.anchor = [Anchor::Centre; 2];
    panel.size = [Size::fixed(width), Size::fixed(height)];
    panel
}

fn choice(view: &View, viewport: Viewport) -> Node {
    let [width, height] = viewport.safe().size;
    let width = (width - 16.0).clamp(0.0, 520.0);
    let roomy = height >= 380.0;
    let card_height = if roomy { 105.0 } else { 67.0 };
    let panel_height = card_height * 2.0 + if roomy { 114.0 } else { 93.0 };
    let mut panel = scope(CHOICE, width, panel_height);
    let mut background = fill(101, INK);
    background.size = [FILL; 2];
    let mut body = stack(102, Axis::Column, if roomy { 12.0 } else { 7.0 });
    body.size = [FILL; 2];
    body.padding = [12.0, 9.0, 12.0, 8.0];
    let mut header = label(103, "Choose an upgrade", 24.0, CREAM);
    header.size[1] = Size::fixed(33.0);
    body.children = vec![header];
    let names = [
        ("Stride", "Move faster"),
        ("Guard", "Take less damage"),
        ("Charge", "Recover sooner"),
        ("Focus", "Hit more precisely"),
    ];
    for (row, pair) in CARDS.as_chunks::<2>().0.iter().enumerate() {
        let mut group = stack(104 + u32::try_from(row).expect("two rows"), Axis::Row, 8.0);
        group.size = [FILL, Size::fixed(card_height)];
        for &id in pair {
            let index = usize::try_from((id - CARDS[0]) / 10).expect("four cards");
            let state = card_style(view, index);
            let (title, detail) = names[index];
            group
                .children
                .push(card(id, title, detail, state, roomy, view.disarmed));
        }
        body.children.push(group);
    }
    body.children
        .push(label(106, choice_prompt(view.disarmed), 12.0, MUTED));
    panel.children = vec![background, body];
    panel
}

fn choice_prompt(disarmed: bool) -> &'static str {
    if disarmed {
        "Get ready"
    } else if cfg!(target_os = "ios") {
        "Tap a card to choose"
    } else {
        "Arrow keys or touch to choose"
    }
}

fn card_style(view: &View, index: usize) -> ButtonState {
    if view.disabled == Some(CARDS[index]) {
        ButtonState::Disabled
    } else {
        view.cards[index]
    }
}

fn card(
    id: NodeId,
    title: &str,
    detail: &str,
    state: ButtonState,
    roomy: bool,
    disarmed: bool,
) -> Node {
    let mut card = Node::new(
        id,
        NodeKind::Button(Button {
            enabled: state != ButtonState::Disabled,
            ..Button::default()
        }),
    );
    card.size = [SHARE, FILL];
    let (background, edge, heading, detail_tint) = card_tints(state, disarmed);
    let mut border = fill(id + 1, edge);
    border.size = [FILL; 2];
    card.padding = [0.0; 4];
    let mut interior = Node::new(id + 2, NodeKind::Overlay);
    interior.size = [FILL; 2];
    interior.padding = [2.0; 4];
    let mut surface = fill(id + 3, background);
    surface.size = [FILL; 2];
    let mut words = stack(id + 4, Axis::Column, if roomy { 5.0 } else { 1.0 });
    words.size = [FILL; 2];
    words.padding = [6.0, if roomy { 22.0 } else { 10.0 }, 6.0, 2.0];
    let mut name = label(id + 5, title, 16.0, heading);
    name.size[1] = Size::fixed(23.0);
    let caption = label(
        id + 6,
        if state == ButtonState::Disabled {
            "Unavailable"
        } else {
            detail
        },
        12.0,
        detail_tint,
    );
    words.children = vec![name, caption];
    interior.children = vec![surface, words];
    card.children = vec![border, interior];
    card
}

fn card_tints(state: ButtonState, disarmed: bool) -> ([f32; 4], [f32; 4], [f32; 4], [f32; 4]) {
    match state {
        ButtonState::Disabled => (
            [0.038, 0.052, 0.063, 1.0],
            [0.12, 0.17, 0.19, 1.0],
            MUTED,
            MUTED,
        ),
        ButtonState::Pressed => ([0.08, 0.47, 0.37, 1.0], TEAL, CREAM, CREAM),
        ButtonState::Focused if !disarmed => ([0.18, 0.13, 0.066, 1.0], GOLD, CREAM, GOLD),
        _ => (
            [0.039, 0.078, 0.092, 1.0],
            [0.13, 0.28, 0.30, 1.0],
            CREAM,
            MUTED,
        ),
    }
}

fn join(view: &View, viewport: Viewport) -> Node {
    let width = (viewport.safe().size[0] - 16.0).clamp(0.0, 460.0);
    let mut panel = scope(JOIN, width, 244.0);
    let mut background = fill(202, INK);
    background.size = [FILL; 2];
    let mut content = stack(203, Axis::Column, 6.0);
    content.padding = [12.0, 11.0, 12.0, 9.0];
    content.size = [FILL; 2];
    let (title, status, reason) = match view.screen {
        Screen::Hosting => (
            "Host a game",
            "Coast outpost",
            "Share this ticket with the other players.",
        ),
        Screen::Joining if view.ticket.is_empty() => (
            "Join a game",
            "Waiting for a ticket",
            if cfg!(target_os = "ios") {
                "Open an invite link."
            } else {
                "Paste a ticket or open an invite link."
            },
        ),
        Screen::Joining => (
            "Join a game",
            "Coast outpost",
            "The ticket is ready. Choose Connect to join.",
        ),
        Screen::Connecting => (
            "Connecting",
            "Coast outpost",
            "Waiting for the host to answer…",
        ),
        Screen::Joined => ("Joined", "Coast outpost", "Players in the lobby"),
        Screen::Refused => (
            "Join refused",
            "Coast outpost",
            "This ticket has expired. Ask the host for a new invite.",
        ),
        Screen::Closed => (
            "Session closed",
            "Coast outpost",
            "The host ended the session. You can join a new game.",
        ),
        Screen::Play | Screen::Choice => unreachable!("join view is selected by screen"),
    };
    let mut heading = label(204, title, 24.0, CREAM);
    heading.size[1] = Size::fixed(38.0);
    let mut subtitle = label(205, status, 16.0, TEAL);
    subtitle.size[1] = Size::fixed(24.0);
    let mut body = label(206, reason, 12.0, CREAM);
    if let NodeKind::Text(text) = &mut body.kind {
        text.options.wrap = true;
    }
    body.size[1] = Size::fixed(36.0);
    content.children = vec![heading, subtitle, body];
    let displayed_ticket = if view.screen == Screen::Hosting {
        HOST_TICKET
    } else {
        &view.ticket
    };
    if matches!(
        view.screen,
        Screen::Hosting | Screen::Joining | Screen::Connecting
    ) && !displayed_ticket.is_empty()
    {
        let mut ticket = label(207, displayed_ticket, 12.0, GOLD);
        if let NodeKind::Text(text) = &mut ticket.kind {
            text.options.elision = Elision::Middle;
        }
        ticket.size[1] = Size::fixed(23.0);
        content.children.push(ticket);
    }
    match view.screen {
        Screen::Hosting => content
            .children
            .push(join_action(COPY, "Copy", view.join_button)),
        Screen::Joining if !view.ticket.is_empty() => {
            content
                .children
                .push(join_action(SUBMIT, "Connect", view.join_button));
        }
        Screen::Joined => {
            for (id, text) in ["Alba · Host", "Bea", "Dara", "Rui"]
                .into_iter()
                .enumerate()
            {
                let mut member = label(
                    230 + u32::try_from(id).expect("four players"),
                    text,
                    12.0,
                    CREAM,
                );
                member.size[1] = Size::fixed(20.0);
                content.children.push(member);
            }
        }
        _ => {}
    }
    panel.children = vec![
        background,
        content,
        Node::new(TICKET_DROP, NodeKind::DropTarget(128)),
    ];
    panel
}

fn join_action(id: NodeId, title: &str, state: ButtonState) -> Node {
    let mut node = Node::new(id, NodeKind::Button(Button::default()));
    node.size = [Size::fixed(112.0), Size::fixed(44.0)];
    let mut fill = fill(id + 1, join_tint(state));
    fill.size = [FILL; 2];
    let mut text = label(id + 2, title, 16.0, CREAM);
    text.anchor = [Anchor::Centre; 2];
    node.children = vec![fill, text];
    node
}

fn join_tint(state: ButtonState) -> [f32; 4] {
    match state {
        ButtonState::Pressed => [0.1, 0.52, 0.42, 1.0],
        ButtonState::Focused => [0.22, 0.35, 0.22, 1.0],
        _ => [0.061, 0.15, 0.17, 1.0],
    }
}

fn fill(id: NodeId, color: [f32; 4]) -> Node {
    Node::new(id, NodeKind::Fill(color))
}

fn stack(id: NodeId, axis: Axis, gap: f64) -> Node {
    Node::new(
        id,
        NodeKind::Stack {
            axis,
            gap,
            align: Anchor::Centre,
        },
    )
}

fn label(id: NodeId, value: &str, points: f32, tint: [f32; 4]) -> Node {
    let mut node = Node::new(
        id,
        NodeKind::Text(Text {
            value: value.into(),
            font: 0,
            options: TextOptions {
                points,
                scale: 1,
                extent: [0; 2],
                alignment: Alignment::Centre,
                wrap: false,
                elision: Elision::End,
                tabular_digits: false,
                outlined: false,
            },
            tint,
            outline: INK,
        }),
    );
    node.size[0] = FILL;
    node
}
