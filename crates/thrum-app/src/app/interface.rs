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

    fn picture_point(&self, point: [f64; 2]) -> [f64; 2] {
        let (Some(window), Some(presented)) = (&self.window, self.presented_output) else {
            return point;
        };
        let bounds = window.inner_size();
        map_picture_point(
            point,
            [bounds.width, bounds.height],
            presented.output.output_extent,
        )
    }

    pub(super) fn interface_cursor(&mut self, point: [f64; 2]) -> Result<(), String> {
        self.interface_input.cursor = Some(point);
        let point = self.picture_point(point);
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
        let position = self
            .interface_input
            .cursor
            .map(|point| self.picture_point(point));
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
        let point = self.picture_point([touch.location.x, touch.location.y]);
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

// Both the pointer and the live bounds are in backing pixels. The layer
// fills those bounds with the last submitted picture during a resize.
fn map_picture_point(point: [f64; 2], bounds: [u32; 2], picture: [u32; 2]) -> [f64; 2] {
    if bounds == picture {
        return point;
    }
    [
        point[0] / f64::from(bounds[0].max(1)) * f64::from(picture[0]),
        point[1] / f64::from(bounds[1].max(1)) * f64::from(picture[1]),
    ]
}

#[cfg(test)]
#[test]
fn picture_point_uses_the_picture_still_on_screen_in_both_directions() {
    let window = [1920, 1080];
    let fullscreen = [3456, 2168];
    let scaled = [2938, 1843];
    assert_eq!(
        map_picture_point([1728.0, 1084.0], fullscreen, window),
        [960.0, 540.0]
    );
    assert_eq!(
        map_picture_point([960.0, 540.0], window, scaled),
        [1469.0, 921.5]
    );
    assert_eq!(
        map_picture_point([96.5, 54.5], window, window),
        [96.5, 54.5]
    );
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
