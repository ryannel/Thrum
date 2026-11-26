use thrum_ui::{Contact, Direction, Event, Intent};

use crate::KeyCode;

/// Direction of an interface clipboard request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardAccess {
    /// Paste text into the interface.
    Read,
    /// Copy the game's string to the clipboard.
    Write,
}

/// Translate a physical key using the default interface bindings.
pub fn interface_key(code: KeyCode, pressed: bool, repeat: bool, shift: bool) -> Event<KeyCode> {
    let contact = Contact::Key(code);
    if !pressed {
        return Event::Release(contact, None);
    }
    if repeat {
        return Event::Repeat(contact);
    }
    let intent = match code {
        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => Intent::Activate,
        KeyCode::Escape => Intent::Cancel,
        KeyCode::Tab if shift => Intent::Previous,
        KeyCode::Tab => Intent::Next,
        KeyCode::ArrowLeft => Intent::Direction(Direction::Left),
        KeyCode::ArrowRight => Intent::Direction(Direction::Right),
        KeyCode::ArrowUp => Intent::Direction(Direction::Up),
        KeyCode::ArrowDown => Intent::Direction(Direction::Down),
        _ => {
            return Event::Press {
                contact,
                position: None,
                intent: None,
            };
        }
    };
    Event::Press {
        contact,
        position: None,
        intent: Some(intent),
    }
}
