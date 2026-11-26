//! The V1 choice and join screen, drawn by the ordinary interface stage.

mod view;
pub(crate) use view::{MenuFrame, paint};

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
mod play;
#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
pub(crate) use play::Menu;

use std::sync::{Arc, Mutex};

use thrum_ui::{ButtonState, NodeId, Scope};

pub(crate) const CHOICE: NodeId = 100;
pub(crate) const JOIN: NodeId = 200;
pub(crate) const TICKET_DROP: NodeId = 201;
pub(crate) const COPY: NodeId = 210;
pub(crate) const SUBMIT: NodeId = 220;
pub(crate) const CARDS: [NodeId; 4] = [110, 120, 130, 140];
pub(crate) const HOST_TICKET: &str = "THRUM-ALPHA-29406-A82EXACL79FTG2OXQ4X65SPPDLX2OUF6KR18PJH5902C01W64NFQVE82NHXQ33DM0-COAST-8432";

#[cfg_attr(
    not(any(target_os = "macos", target_arch = "wasm32")),
    expect(
        dead_code,
        reason = "only the window and browser drive the menu states in this work package"
    )
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Play,
    Choice,
    Hosting,
    Joining,
    Connecting,
    Joined,
    Refused,
    Closed,
}

impl Screen {
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    pub(crate) fn scope(self) -> Option<NodeId> {
        match self {
            Self::Play => None,
            Self::Choice => Some(CHOICE),
            _ => Some(JOIN),
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn named(value: &str) -> Option<Self> {
        Some(match value {
            "play" => Self::Play,
            "choice" => Self::Choice,
            "hosting" => Self::Hosting,
            "joining" => Self::Joining,
            "connecting" => Self::Connecting,
            "joined" => Self::Joined,
            "refused" => Self::Refused,
            "closed" => Self::Closed,
            _ => return None,
        })
    }
}

pub(crate) const fn policy(id: NodeId) -> Scope {
    Scope {
        initial: match id {
            CHOICE => Some(CARDS[0]),
            _ => Some(SUBMIT),
        },
        exclusive: true,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct View {
    pub(crate) screen: Screen,
    pub(crate) cards: [ButtonState; 4],
    pub(crate) join_button: ButtonState,
    pub(crate) disabled: Option<NodeId>,
    pub(crate) disarmed: bool,
    pub(crate) ticket: Arc<str>,
    pub(crate) edit: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            screen: Screen::Play,
            cards: [ButtonState::Idle; 4],
            join_button: ButtonState::Idle,
            disabled: None,
            disarmed: false,
            ticket: Arc::from(""),
            edit: false,
        }
    }
}

/// Game-owned presentation packet crossing to the render owner.
#[derive(Clone, Default)]
pub(crate) struct MenuView(Arc<Mutex<Arc<View>>>);

impl MenuView {
    pub(crate) fn get(&self) -> Arc<View> {
        Arc::clone(&self.0.lock().expect("menu view mailbox"))
    }

    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    fn set(&self, view: &View) {
        let mut packet = self.0.lock().expect("menu view mailbox");
        if **packet != *view {
            *packet = Arc::new(view.clone());
        }
    }
}
