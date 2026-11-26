use std::time::Duration;

use thrum_app::KeyCode;
use thrum_ui::{Contact, Event, Routed, Router};

/// Contact routing for the noninteractive status display and the camera beneath it.
#[derive(Default)]
pub struct Input {
    router: Router<KeyCode>,
    drag: Option<(Contact<KeyCode>, [f64; 2])>,
    pan: [f64; 2],
    keys: u64,
    mouse: u64,
    touches: u64,
}

impl Input {
    /// Pass first-chance input through the interface to the camera.
    pub fn event(
        &mut self,
        event: Event<KeyCode>,
        now: Duration,
        scale: u32,
    ) -> Option<Routed<KeyCode>> {
        let event = points(event, f64::from(scale.max(1)));
        let routed = self.router.route(event, now)?;
        match &routed {
            Routed::Game(Event::Press {
                contact: Contact::Key(key),
                ..
            }) => {
                let delta = match key {
                    KeyCode::ArrowLeft => [-5.0, 0.0],
                    KeyCode::ArrowRight => [5.0, 0.0],
                    KeyCode::ArrowUp => [0.0, 5.0],
                    KeyCode::ArrowDown => [0.0, -5.0],
                    _ => [0.0; 2],
                };
                self.keys += 1;
                self.pan = [self.pan[0] + delta[0], self.pan[1] + delta[1]];
            }
            Routed::Game(Event::Press {
                contact,
                position: Some(position),
                ..
            }) => {
                match contact {
                    Contact::Pointer(_) => self.mouse += 1,
                    Contact::Touch(_) => self.touches += 1,
                    Contact::Key(_) => {}
                }
                if self.drag.is_none() {
                    self.drag = Some((*contact, *position));
                }
            }
            Routed::Game(Event::Move(contact, point)) => {
                if let Some((owner, previous)) = &mut self.drag
                    && *owner == *contact
                {
                    self.pan[0] += (previous[0] - point[0]) * 0.22;
                    self.pan[1] += (point[1] - previous[1]) * 0.22;
                    *previous = *point;
                }
            }
            Routed::Game(Event::Release(contact, _) | Event::Cancel(contact)) => {
                if self.drag.is_some_and(|(owner, _)| owner == *contact) {
                    self.drag = None;
                }
            }
            Routed::Interrupted | Routed::Game(Event::Interrupted) => self.drag = None,
            _ => {}
        }
        Some(routed)
    }

    /// Desired camera offset in world units.
    #[must_use]
    pub const fn pan(&self) -> [f64; 2] {
        self.pan
    }

    /// Count key, pointer and touch presses delivered to the game.
    #[must_use]
    pub const fn presses(&self) -> [u64; 3] {
        [self.keys, self.mouse, self.touches]
    }
}

fn points(event: Event<KeyCode>, scale: f64) -> Event<KeyCode> {
    let point = |position: [f64; 2]| position.map(|value| value / scale);
    match event {
        Event::Press {
            contact,
            position,
            intent,
        } => Event::Press {
            contact,
            position: position.map(point),
            intent,
        },
        Event::Move(contact, position) => Event::Move(contact, point(position)),
        Event::Release(contact, position) => Event::Release(contact, position.map(point)),
        Event::Hover(position) => Event::Hover(point(position)),
        event => event,
    }
}
