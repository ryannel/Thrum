use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use thrum_app::{KeyCode, interface_key, offscreen::Offscreen};
use thrum_ui::ButtonState;

use crate::interface_menu::{CARDS, HOST_TICKET, Menu, Screen, View};

use super::{HostedGame, WindowGame};

type Driver = Offscreen<HostedGame<WindowGame>>;

pub(super) fn setup(state: Option<&str>) -> Result<Menu, String> {
    let mut menu = Menu::new(true);
    let Some(state) = state else {
        return Ok(menu);
    };
    let mut view = View::default();
    let (screen, style) = state.split_once(':').unwrap_or((state, ""));
    view.screen = Screen::named(screen).ok_or("unknown menu capture screen")?;
    if view.screen == Screen::Choice {
        match style {
            "" | "idle" => {}
            "pressed" => view.cards[1] = ButtonState::Pressed,
            "disabled" => {
                view.cards[0] = ButtonState::Focused;
                view.disabled = Some(CARDS[3]);
            }
            "disarmed" => view.disarmed = true,
            card => {
                let index = card.parse::<usize>().map_err(|error| error.to_string())?;
                *view
                    .cards
                    .get_mut(index)
                    .ok_or("unknown choice capture card")? = ButtonState::Focused;
            }
        }
    } else if style == "ticket" || view.screen == Screen::Connecting {
        view.ticket = Arc::from(HOST_TICKET);
    }
    menu.hold(view);
    Ok(menu)
}

pub(super) fn warm(driver: &mut Driver) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let report = driver.step().map_err(|error| error.to_string())?;
        drop(driver.completed()?);
        if report.submitted && driver.host().source().submitted_menu().is_some() {
            // T2 primes an ordinary submission immediately after this warm-up.
            while driver.pending_frames() > 0 {
                drop(driver.completed()?);
                if Instant::now() >= deadline {
                    return Err("menu warm-up did not retire".to_owned());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("menu font and layout did not submit".to_owned());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

const STEPS: [(u64, KeyCode, bool); 12] = [
    (1200, KeyCode::KeyD, true),
    (1700, KeyCode::KeyM, true),
    (1730, KeyCode::KeyM, false),
    (2050, KeyCode::ArrowRight, true),
    (2100, KeyCode::ArrowRight, false),
    (2380, KeyCode::ArrowDown, true),
    (2430, KeyCode::ArrowDown, false),
    (2700, KeyCode::ArrowLeft, true),
    (2750, KeyCode::ArrowLeft, false),
    (3020, KeyCode::Enter, true),
    (3070, KeyCode::Enter, false),
    (3690, KeyCode::KeyD, false),
];

#[derive(Default)]
pub(super) struct Take {
    next: usize,
    pub(super) events: Vec<Value>,
}

impl Take {
    pub(super) fn dispatch(
        &mut self,
        driver: &mut Driver,
        origin: Instant,
        scenario: Duration,
    ) -> Result<(), String> {
        while let Some(&(ms, key, down)) = STEPS.get(self.next) {
            if scenario < Duration::from_millis(ms) {
                break;
            }
            let index = driver.last_submission();
            let requested = origin.elapsed().as_nanos();
            driver
                .interface_input(interface_key(key, down, false, false))
                .map_err(|error| error.to_string())?;
            self.next += 1;
            self.events.push(json!({
                "code": format!("{key:?}"), "down": down, "request_ns": requested,
                "seen_through_submission": index,
                "accepted_ns": origin.elapsed().as_nanos(),
                "game": driver.game().0.menu.as_ref().map(Menu::report)
            }));
        }
        Ok(())
    }
}
