//! UIKit output geometry and first-chance input routing.

use std::ffi::c_void;

use thrum_ui::{Contact, Event, Routed};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use super::{MobileGame, Shell, gesture::Phase};
use crate::{KeyCode, OutputMetrics};

#[expect(
    unsafe_code,
    reason = "read the borrowed UIKit view synchronously on its main thread"
)]
pub(super) fn output(window: &Window, extent: [u32; 2]) -> Option<OutputMetrics> {
    unsafe extern "C" {
        fn thrum_ios_safe_area(view: *mut c_void, insets: *mut f64);
    }
    let RawWindowHandle::UiKit(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    let mut insets = [0.0; 4];
    // SAFETY: The shell owns this live window and the output buffer outlives the call.
    unsafe { thrum_ios_safe_area(handle.ui_view.as_ptr(), insets.as_mut_ptr()) };
    Some(OutputMetrics {
        extent,
        density: window.scale_factor(),
        insets,
    })
}

impl<G: MobileGame> Shell<G> {
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
        if let Some(input) = &mut self.input {
            G::event(input, event, self.origin.elapsed());
        }
        self.interface_input()
    }

    pub(super) fn interface_input(&mut self) -> Result<(), String> {
        let Some(input) = &mut self.input else {
            return Ok(());
        };
        let ui_scale = self.interface_scale;
        let density = self
            .window
            .as_ref()
            .map_or(1.0, |window| window.scale_factor());
        let physical = |point: [f64; 2]| point.map(|axis| axis * ui_scale);
        let mut axes = None;
        for routed in G::take_input(input) {
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
