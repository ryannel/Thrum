//! UIKit output geometry and first-chance input routing for the V1 interface.

use std::{
    ffi::{CStr, c_char, c_void},
    path::PathBuf,
};

use thrum_app::KeyCode;
use thrum_ui::{Contact, Event, Routed};
use v1_game::mobile_interface::InterfaceFrame;
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use super::{SHELL, Shell, ShellEvent};
use crate::gesture::Phase;

#[derive(Clone, Copy)]
pub(super) struct Output {
    pub(super) density: f64,
    pub(super) insets: [f64; 4],
}

#[derive(Debug)]
pub(super) struct Invite {
    pub(super) ticket: String,
    pub(super) font_store: PathBuf,
}

#[expect(
    unsafe_code,
    reason = "read the borrowed UIKit view synchronously on its main thread"
)]
pub(super) fn output(window: &Window) -> Option<Output> {
    unsafe extern "C" {
        fn thrum_v1_safe_area(view: *mut c_void, insets: *mut f64);
    }
    let RawWindowHandle::UiKit(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    let mut insets = [0.0; 4];
    // SAFETY: The shell owns this live window and the output buffer outlives the call.
    unsafe { thrum_v1_safe_area(handle.ui_view.as_ptr(), insets.as_mut_ptr()) };
    Some(Output {
        density: window.scale_factor(),
        insets,
    })
}

#[expect(
    unsafe_code,
    reason = "Swift forwards borrowed ticket and font store strings through this C ABI"
)]
#[unsafe(no_mangle)]
extern "C" fn thrum_v1_open_invite(
    bytes: *const u8,
    len: usize,
    connecting: i32,
    font_store: *const c_char,
) {
    let ticket = if bytes.is_null() || len == 0 {
        None
    } else {
        // SAFETY: Swift owns the buffer for the duration of this call.
        let bytes = unsafe { std::slice::from_raw_parts(bytes, len) };
        std::str::from_utf8(bytes).ok().map(str::to_owned)
    };
    let invite = ticket.filter(|_| !font_store.is_null()).and_then(|ticket| {
        // SAFETY: Swift keeps this NUL-terminated path live through the call.
        let store = unsafe { CStr::from_ptr(font_store) }.to_str().ok()?;
        Some(Invite {
            ticket,
            font_store: PathBuf::from(store),
        })
    });
    let event = if connecting != 0 {
        Some(ShellEvent::SceneConnected(invite))
    } else {
        invite.map(ShellEvent::Invite)
    };
    if let (Some(shell), Some(event)) = (SHELL.get(), event) {
        drop(shell.send_event(event));
    }
}

impl Shell {
    pub(super) fn deliver_invite(&mut self) -> Result<(), String> {
        let Some(menu) = &mut self.interface_menu else {
            return Ok(());
        };
        let Some(invite) = self.pending_invite.take() else {
            return Ok(());
        };
        menu.open_invite(invite.ticket, self.origin.elapsed());
        self.interface_input()?;
        self.write();
        Ok(())
    }

    pub(super) fn interface_touch(
        &mut self,
        id: u64,
        phase: Phase,
        location: [f64; 2],
    ) -> Result<(), String> {
        let contact = Contact::Touch(id);
        let event = match phase {
            Phase::Started => Event::Press {
                contact,
                position: Some(location),
                intent: None,
            },
            Phase::Moved => Event::Move(contact, location),
            Phase::Ended => Event::Release(contact, Some(location)),
            Phase::Cancelled => Event::Cancel(contact),
        };
        self.interface_event(event)
    }

    pub(super) fn interface_event(&mut self, event: Event<KeyCode>) -> Result<(), String> {
        if let Some(menu) = &mut self.interface_menu {
            menu.event(event, self.origin.elapsed());
        }
        self.interface_input()
    }

    pub(super) fn interface_submitted(&mut self, frame: &InterfaceFrame) -> Result<bool, String> {
        let Some(menu) = &mut self.interface_menu else {
            return Ok(false);
        };
        self.interface_scale = f64::from(frame.viewport().scale);
        let redraw = menu.submitted(frame, self.witness.presented, self.origin.elapsed());
        self.interface_input()?;
        Ok(redraw)
    }

    pub(super) fn interface_input(&mut self) -> Result<(), String> {
        let Some(menu) = &mut self.interface_menu else {
            return Ok(());
        };
        let ui_scale = self.interface_scale;
        let density = self
            .window
            .as_ref()
            .map_or(1.0, |window| window.scale_factor());
        let physical = |point: [f64; 2]| point.map(|axis| axis * ui_scale);
        let mut axes = None;
        for routed in menu.input() {
            let next = match routed {
                Routed::Game(Event::Press {
                    contact: Contact::Touch(id),
                    position: Some(point),
                    ..
                }) => self
                    .gesture
                    .touch(id, Phase::Started, physical(point), density),
                Routed::Game(Event::Move(Contact::Touch(id), point)) => {
                    self.gesture
                        .touch(id, Phase::Moved, physical(point), density)
                }
                Routed::Game(Event::Release(Contact::Touch(id), Some(point))) => self
                    .gesture
                    .touch(id, Phase::Ended, physical(point), density),
                Routed::Game(Event::Cancel(Contact::Touch(id))) => {
                    self.gesture.touch(id, Phase::Cancelled, [0.0; 2], density)
                }
                Routed::Interrupted => self.gesture.interrupt(),
                Routed::Resumed(held) => {
                    let touches =
                        held.into_iter()
                            .filter_map(|held| match (held.contact, held.position) {
                                (Contact::Touch(id), Some(point)) => Some((id, physical(point))),
                                _ => None,
                            });
                    self.gesture.resume(touches, density)
                }
                _ => None,
            };
            axes = next.or(axes);
        }
        if let Some(axes) = axes {
            self.send_axes(axes)?;
        }
        Ok(())
    }
}
