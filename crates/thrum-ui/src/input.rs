//! Interface events, focus scopes and contact ownership.

mod focus;

use std::{collections::BTreeMap, time::Duration};

use crate::{Box2, NodeId};

/// The four directions a control can name a neighbour in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// Toward the left.
    Left,
    /// Toward the right.
    Right,
    /// Toward the top.
    Up,
    /// Toward the bottom.
    Down,
}

/// The action bound to a key by the game.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Intent {
    /// Move focus geometrically.
    Direction(Direction),
    /// Move focus forward in tree order.
    Next,
    /// Move focus backward in tree order.
    Previous,
    /// Activate the focused button.
    Activate,
    /// Ask the game to close the scope.
    Cancel,
}

/// Identity of a contact, from its press until its release.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Contact<K> {
    /// Physical key.
    Key(K),
    /// Pointer button number.
    Pointer(u16),
    /// Touch identity from the shell.
    Touch(u64),
}

/// Origin of an entire text drop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextSource {
    /// Native launch argument.
    Launch,
    /// Link or browser location fragment.
    Link,
    /// System paste.
    Paste,
}

/// An input as translated by the shell and bound by the game.
#[derive(Clone, Debug, PartialEq)]
pub enum Event<K> {
    /// New press. Positions use the displayed layout's point grid.
    Press {
        /// Contact that went down.
        contact: Contact<K>,
        /// Position for pointer and touch contacts.
        position: Option<[f64; 2]>,
        /// Current key binding, if any.
        intent: Option<Intent>,
    },
    /// A platform key repeat, distinct from a new press.
    Repeat(Contact<K>),
    /// Movement of a held contact.
    Move(Contact<K>, [f64; 2]),
    /// Movement of a pointer with no button held.
    Hover([f64; 2]),
    /// A contact went up, with its final position when available.
    Release(Contact<K>, Option<[f64; 2]>),
    /// A contact was cancelled by the platform.
    Cancel(Contact<K>),
    /// Focus was lost or the application went into the background.
    Interrupted,
    /// Text supplied whole by the shell.
    Text(String, TextSource),
}

/// A contact presented to the game as still held when a modal closes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held<K> {
    /// Contact identity.
    pub contact: Contact<K>,
    /// Most recent position for a spatial contact.
    pub position: Option<[f64; 2]>,
}

/// What the game receives after routing.
#[derive(Clone, Debug, PartialEq)]
pub enum Routed<K> {
    /// Event the interface passed through.
    Game(Event<K>),
    /// Release every contact the game currently holds.
    Interrupted,
    /// Acquire these contacts at their current positions.
    Resumed(Vec<Held<K>>),
    /// A button was worked.
    Activated(NodeId),
    /// The top scope was cancelled.
    Cancelled(NodeId),
    /// Whole text accepted by the designated node.
    Text(NodeId, String, TextSource),
}

/// Per-button input policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Button {
    /// Whether activation is permitted.
    pub enabled: bool,
    /// Explicit neighbours in left, right, up, down order.
    pub neighbours: [Option<NodeId>; 4],
}

impl Default for Button {
    fn default() -> Self {
        Self {
            enabled: true,
            neighbours: [None; 4],
        }
    }
}

/// Input policy of a scope node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    /// Preferred control on entry and when focus is invalidated.
    pub initial: Option<NodeId>,
    /// Interrupt and suspend game contacts while the scope is open.
    pub exclusive: bool,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            initial: None,
            exclusive: true,
        }
    }
}

/// Visual state a game uses to choose a button's style.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonState {
    /// No active emphasis.
    Idle,
    /// The control has keyboard or pointer focus.
    Focused,
    /// A matching contact is held inside the button.
    Pressed,
    /// The button cannot be activated.
    Disabled,
}

#[derive(Clone, Debug)]
pub(crate) struct Control {
    pub id: NodeId,
    pub bounds: Box2,
    pub button: Button,
    pub scope: Option<NodeId>,
}

/// The input geometry corresponding to an immutable paint list.
#[derive(Clone, Debug, Default)]
pub struct InputLayout {
    pub(crate) controls: Vec<Control>,
    pub(crate) drops: Vec<(Option<NodeId>, NodeId, usize)>,
}

impl InputLayout {
    /// The unrounded drawn box of a visible control.
    pub fn bounds(&self, id: NodeId) -> Option<Box2> {
        self.controls
            .iter()
            .find(|control| control.id == id)
            .map(|control| control.bounds)
    }
}

struct OpenScope {
    id: NodeId,
    policy: Scope,
    focus: Option<NodeId>,
    armed_at: Duration,
}

#[derive(Clone, Copy)]
enum Owner {
    Game,
    Suspended,
    Interface {
        scope: Option<NodeId>,
        button: Option<NodeId>,
        intent: Option<Intent>,
        worked: bool,
    },
}

struct Press {
    owner: Owner,
    position: Option<[f64; 2]>,
}

/// Routes complete contacts and keeps focus on the layout last submitted.
pub struct Router<K> {
    shown: InputLayout,
    scopes: Vec<OpenScope>,
    presses: BTreeMap<Contact<K>, Press>,
    show_focus: bool,
    refused: u64,
    clipboard_copy: Option<String>,
}

impl<K> Default for Router<K> {
    fn default() -> Self {
        Self {
            shown: InputLayout::default(),
            scopes: Vec::new(),
            presses: BTreeMap::new(),
            show_focus: true,
            refused: 0,
            clipboard_copy: None,
        }
    }
}

impl<K: Copy + Ord> Router<K> {
    /// Switch input geometry when the frame that drew it has been submitted.
    pub fn submitted(&mut self, layout: InputLayout) {
        let previous = std::mem::replace(&mut self.shown, layout);
        let shown = &self.shown;
        let successor = self.focus().and_then(|focus| {
            previous
                .controls
                .iter()
                .skip_while(|control| control.id != focus)
                .skip(1)
                .find(|control| eligible(shown, self.top_id(), control.id))
                .map(|control| control.id)
        });
        for press in self.presses.values_mut() {
            if let Owner::Interface { button, scope, .. } = &mut press.owner
                && button.is_some_and(|id| !eligible(shown, *scope, id))
            {
                *button = None;
            }
        }
        self.reconcile_focus(successor);
    }

    /// Open a declared scope immediately. Supply its policy from the current tree.
    /// Returns an interruption only when entering the first exclusive scope.
    pub fn open(&mut self, id: NodeId, policy: Scope, armed_at: Duration) -> Option<Routed<K>> {
        if self.scopes.iter().any(|scope| scope.id == id) {
            return None;
        }
        let interrupts = policy.exclusive && !self.exclusive();
        self.cancel_scope_presses();
        self.scopes.push(OpenScope {
            id,
            policy,
            focus: None,
            armed_at,
        });
        self.show_focus = true;
        self.reconcile_focus(None);
        if interrupts {
            for press in self.presses.values_mut() {
                if matches!(press.owner, Owner::Game) {
                    press.owner = Owner::Suspended;
                }
            }
            return Some(Routed::Interrupted);
        }
        None
    }

    /// Close the top scope and restore its predecessor's focus.
    pub fn close(&mut self, id: NodeId) -> Option<Routed<K>> {
        if self.top_id() != Some(id) {
            return None;
        }
        let was_exclusive = self.exclusive();
        self.cancel_scope_presses();
        self.scopes.pop();
        self.reconcile_focus(None);
        (was_exclusive && !self.exclusive()).then(|| {
            let held = self.presses.iter_mut().filter_map(|(&contact, press)| {
                if matches!(
                    press.owner,
                    Owner::Suspended | Owner::Interface { worked: false, .. }
                ) {
                    press.owner = Owner::Game;
                    Some(Held {
                        contact,
                        position: press.position,
                    })
                } else {
                    None
                }
            });
            Routed::Resumed(held.collect())
        })
    }

    /// The focused identity, including focus hidden by touch modality.
    pub fn focus(&self) -> Option<NodeId> {
        self.scopes.last().and_then(|scope| scope.focus)
    }

    /// Number of drops rejected by the top scope's designated receiver.
    pub const fn refused_drops(&self) -> u64 {
        self.refused
    }

    /// Request that the shell copy an entire string.
    pub fn request_copy(&mut self, value: String) {
        self.clipboard_copy = Some(value);
    }

    /// Take the pending copy request for the shell.
    pub fn take_copy(&mut self) -> Option<String> {
        self.clipboard_copy.take()
    }

    /// Route one event at the shell's monotonic time.
    pub fn route(&mut self, event: Event<K>, now: Duration) -> Option<Routed<K>> {
        match event {
            Event::Interrupted => {
                self.presses.clear();
                Some(Routed::Interrupted)
            }
            Event::Text(value, source) => self.text(value, source, now),
            Event::Press {
                contact,
                position,
                intent,
            } => self.press(contact, position, intent, now),
            Event::Repeat(contact) => self.repeat(contact, now),
            Event::Move(contact, position) => self.move_contact(contact, position, now),
            Event::Hover(position) => self.hover(position, now),
            Event::Release(contact, position) => self.release(contact, position, now),
            Event::Cancel(contact) => {
                let press = self.presses.remove(&contact)?;
                matches!(press.owner, Owner::Game).then_some(Routed::Game(event))
            }
        }
    }

    fn exclusive(&self) -> bool {
        self.scopes.iter().any(|scope| scope.policy.exclusive)
    }

    fn top_id(&self) -> Option<NodeId> {
        self.scopes.last().map(|scope| scope.id)
    }

    fn armed(&self, now: Duration) -> bool {
        self.scopes.last().is_none_or(|scope| now >= scope.armed_at)
    }

    fn cancel_scope_presses(&mut self) {
        for press in self.presses.values_mut() {
            if let Owner::Interface { button, intent, .. } = &mut press.owner {
                *button = None;
                *intent = None;
            }
        }
    }

    fn press(
        &mut self,
        contact: Contact<K>,
        position: Option<[f64; 2]>,
        intent: Option<Intent>,
        now: Duration,
    ) -> Option<Routed<K>> {
        if self.presses.contains_key(&contact) {
            return None;
        }
        let binding = intent;
        let intent =
            intent.filter(|_| matches!(contact, Contact::Key(_)) && self.top_id().is_some());
        let hit = position.and_then(|point| self.hit(point));
        let disarmed = !self.armed(now);
        let owner = if self.exclusive() || disarmed || hit.is_some() || intent.is_some() {
            let button = hit.filter(|id| !disarmed && eligible(&self.shown, self.top_id(), *id));
            if !disarmed {
                self.show_focus = !matches!(contact, Contact::Touch(_));
                if !matches!(contact, Contact::Key(_)) {
                    self.set_focus(button);
                }
            }
            Owner::Interface {
                scope: self.top_id(),
                button,
                intent: intent.filter(|_| !disarmed),
                worked: hit.is_some() || intent.is_some(),
            }
        } else {
            Owner::Game
        };
        self.presses.insert(contact, Press { owner, position });
        if matches!(owner, Owner::Game) {
            Some(Routed::Game(Event::Press {
                contact,
                position,
                intent: binding,
            }))
        } else if let Owner::Interface {
            intent: Some(intent),
            ..
        } = owner
        {
            self.apply_intent(intent)
        } else {
            None
        }
    }

    fn repeat(&mut self, contact: Contact<K>, now: Duration) -> Option<Routed<K>> {
        match self.presses.get(&contact)?.owner {
            Owner::Game if self.armed(now) => Some(Routed::Game(Event::Repeat(contact))),
            Owner::Interface {
                scope,
                intent: Some(intent),
                ..
            } if scope == self.top_id()
                && self.armed(now)
                && !matches!(intent, Intent::Activate | Intent::Cancel) =>
            {
                self.show_focus = true;
                self.apply_intent(intent)
            }
            _ => None,
        }
    }

    fn move_contact(
        &mut self,
        contact: Contact<K>,
        position: [f64; 2],
        now: Duration,
    ) -> Option<Routed<K>> {
        let armed = self.armed(now);
        let hit = self.hit(position);
        let shown = &self.shown;
        let press = self.presses.get_mut(&contact)?;
        press.position = Some(position);
        match &mut press.owner {
            Owner::Game => Some(Routed::Game(Event::Move(contact, position))),
            Owner::Interface { button, .. } => {
                if button.is_some_and(|id| !focus::contains(shown, id, position)) {
                    *button = None;
                }
                if armed && matches!(contact, Contact::Pointer(_)) {
                    self.show_focus = true;
                    self.set_focus(hit);
                }
                None
            }
            Owner::Suspended => None,
        }
    }

    fn hover(&mut self, position: [f64; 2], now: Duration) -> Option<Routed<K>> {
        if !self.armed(now) {
            return None;
        }
        let hit = self.hit(position);
        self.show_focus = true;
        self.set_focus(hit);
        (hit.is_none() && !self.exclusive()).then_some(Routed::Game(Event::Hover(position)))
    }

    fn release(
        &mut self,
        contact: Contact<K>,
        position: Option<[f64; 2]>,
        now: Duration,
    ) -> Option<Routed<K>> {
        let press = self.presses.remove(&contact)?;
        match press.owner {
            Owner::Game => Some(Routed::Game(Event::Release(contact, position))),
            Owner::Interface {
                scope,
                button: Some(id),
                ..
            } if self.top_id() == scope
                && self.armed(now)
                && position
                    .or(press.position)
                    .is_some_and(|point| focus::contains(&self.shown, id, point)) =>
            {
                Some(Routed::Activated(id))
            }
            _ => None,
        }
    }

    fn text(&mut self, value: String, source: TextSource, now: Duration) -> Option<Routed<K>> {
        if !self.armed(now) {
            return None;
        }
        if let Some(&(_, id, limit)) = self
            .shown
            .drops
            .iter()
            .find(|drop| drop.0.is_some() && drop.0 == self.top_id())
        {
            if value.chars().take(limit.saturating_add(1)).count() > limit
                || value.chars().any(char::is_control)
            {
                self.refused += 1;
                None
            } else {
                Some(Routed::Text(id, value, source))
            }
        } else {
            Some(Routed::Game(Event::Text(value, source)))
        }
    }
}

fn eligible(layout: &InputLayout, scope: Option<NodeId>, id: NodeId) -> bool {
    layout
        .controls
        .iter()
        .any(|control| control.scope == scope && control.id == id && control.button.enabled)
}
