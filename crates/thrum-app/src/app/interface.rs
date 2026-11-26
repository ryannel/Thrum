use std::collections::BTreeSet;

use objc2_app_kit::{NSPasteboard, NSPasteboardAccessBehavior, NSPasteboardTypeString};
use objc2_foundation::NSString;
use thrum_ui::{Contact, Event, TextSource};
use winit::{
    event::{ElementState, MouseButton, Touch, TouchPhase},
    keyboard::ModifiersState,
};

use super::{App, WindowGame};
use crate::{ClipboardAccess, KeyCode, KeyResponse};

#[derive(Default)]
pub(super) struct WindowInput {
    pub(super) modifiers: ModifiersState,
    cursor: Option<[f64; 2]>,
    buttons: BTreeSet<u16>,
    paste_held: bool,
}

impl<G: WindowGame + 'static> App<G> {
    pub(super) fn interface_event(&mut self, event: Event<KeyCode>) -> Result<(), String> {
        let reply = self
            .game
            .interface_input(event, self.origin.elapsed())
            .map_err(|error| error.to_string())?;
        if let Some(text) = self.game.take_clipboard_copy() {
            self.game
                .clipboard_result(ClipboardAccess::Write, copy(&text));
        }
        match reply {
            KeyResponse::Ignored => Ok(()),
            KeyResponse::Edited => self.controls_edited(),
            KeyResponse::Input => {
                self.input_changed();
                Ok(())
            }
        }
    }

    pub(super) fn interface_key(
        &mut self,
        code: KeyCode,
        state: ElementState,
        repeat: bool,
        unattended: bool,
    ) -> Result<(), String> {
        let pressed = state == ElementState::Pressed;
        if code == KeyCode::KeyV {
            if !pressed && std::mem::take(&mut self.interface_input.paste_held) {
                return Ok(());
            }
            if pressed && self.interface_input.paste_held {
                return Ok(());
            }
            if pressed && !repeat && self.interface_input.modifiers.super_key() {
                self.interface_input.paste_held = true;
                let text = paste(unattended);
                self.game
                    .clipboard_result(ClipboardAccess::Read, text.is_some());
                return text.map_or(Ok(()), |text| {
                    self.interface_event(Event::Text(text, TextSource::Paste))
                });
            }
        }
        self.interface_event(crate::interface_key(
            code,
            pressed,
            repeat,
            self.interface_input.modifiers.shift_key(),
        ))
    }

    pub(super) fn interface_cursor(&mut self, point: [f64; 2]) -> Result<(), String> {
        self.interface_input.cursor = Some(point);
        if self.interface_input.buttons.is_empty() {
            return self.interface_event(Event::Hover(point));
        }
        let buttons = self.interface_input.buttons.clone();
        for button in buttons {
            self.interface_event(Event::Move(Contact::Pointer(button), point))?;
        }
        Ok(())
    }

    pub(super) fn interface_cursor_left(&mut self) -> Result<(), String> {
        if !self.interface_input.buttons.is_empty() {
            self.interface_cursor([-1e6; 2])?;
        }
        self.interface_input.cursor = None;
        Ok(())
    }

    pub(super) fn interface_mouse(
        &mut self,
        button: MouseButton,
        state: ElementState,
    ) -> Result<(), String> {
        let button = match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Back => 3,
            MouseButton::Forward => 4,
            MouseButton::Other(number) => number.saturating_add(5),
        };
        let contact = Contact::Pointer(button);
        let position = self.interface_input.cursor;
        let event = if state == ElementState::Pressed {
            self.interface_input.buttons.insert(button);
            Event::Press {
                contact,
                position,
                intent: None,
            }
        } else {
            self.interface_input.buttons.remove(&button);
            Event::Release(contact, position)
        };
        self.interface_event(event)
    }

    pub(super) fn interface_touch(&mut self, touch: Touch) -> Result<(), String> {
        let contact = Contact::Touch(touch.id);
        let point = [touch.location.x, touch.location.y];
        self.interface_event(match touch.phase {
            TouchPhase::Started => Event::Press {
                contact,
                position: Some(point),
                intent: None,
            },
            TouchPhase::Moved => Event::Move(contact, point),
            TouchPhase::Ended => Event::Release(contact, Some(point)),
            TouchPhase::Cancelled => Event::Cancel(contact),
        })
    }
}

#[expect(
    unsafe_code,
    reason = "AppKit's exported NSString constant has static lifetime"
)]
fn paste(unattended: bool) -> Option<String> {
    let board = NSPasteboard::generalPasteboard();
    if unattended && board.accessBehavior() != NSPasteboardAccessBehavior::AlwaysAllow {
        return None;
    }
    // SAFETY: the AppKit framework owns this immutable type constant.
    board
        .stringForType(unsafe { NSPasteboardTypeString })
        .map(|text| text.to_string())
}

#[expect(
    unsafe_code,
    reason = "AppKit's exported NSString constant has static lifetime"
)]
fn copy(text: &str) -> bool {
    let board = NSPasteboard::generalPasteboard();
    board.clearContents();
    // SAFETY: the AppKit framework owns this immutable type constant.
    board.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString })
}
