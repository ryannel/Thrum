//! V1 game-side interface state for the iOS shell.

use std::{sync::Arc, time::Duration};

use thrum_app::KeyCode;
use thrum_ui::{Event, Routed, TextSource};

use crate::{
    GameScene,
    interface_menu::{Menu, MenuFrame, MenuView, Screen},
};

/// The mobile scene's input and scope state, owned by the event thread.
pub struct MobileInterface {
    menu: Menu,
    shown: Option<MenuFrame>,
}

/// Game-owned view state read by the render owner.
#[derive(Clone)]
pub struct InterfaceView(MenuView);

/// Input layout and view from a single submitted interface frame.
pub struct InterfaceFrame(MenuFrame);

impl MobileInterface {
    /// Start in the play view.
    pub fn new(proof: bool) -> Self {
        Self {
            menu: Menu::new(proof),
            shown: None,
        }
    }

    /// Share presentation state with the render owner.
    pub fn view(&self) -> InterfaceView {
        InterfaceView(self.menu.mailbox())
    }

    /// Advance scope timers.
    pub fn tick(&mut self, now: Duration) {
        self.menu.tick(now);
    }

    /// Route platform input in drawable pixels.
    pub fn event(&mut self, event: Event<KeyCode>, now: Duration) {
        self.menu.event(event, now);
    }

    /// Take input returned to the game. Spatial positions are in interface points.
    pub fn input(&mut self) -> impl Iterator<Item = Routed<KeyCode>> + '_ {
        self.menu.mobile_input.drain(..)
    }

    /// Accept a whole invited ticket when the join layout is visible.
    pub fn open_invite(&mut self, ticket: String, now: Duration) {
        self.menu.launch(ticket, TextSource::Link, now);
    }

    /// Open or close the choice while the simulator test holds a real contact.
    pub fn set_choice(&mut self, open: bool, now: Duration) {
        self.menu
            .screen(if open { Screen::Choice } else { Screen::Play }, now);
    }

    /// Route future touches against the frame that reached the surface.
    pub fn submitted(&mut self, frame: &InterfaceFrame, index: u64, now: Duration) -> bool {
        let changed = self.shown.as_ref().is_none_or(|previous| {
            previous.viewport != frame.0.viewport || !Arc::ptr_eq(&previous.view, &frame.0.view)
        });
        if changed {
            self.shown = Some(frame.0.clone());
        }
        self.menu.submitted(&frame.0, index, None, now);
        changed
    }

    /// The game and router's qualification record.
    pub fn report(&self) -> serde_json::Value {
        self.menu.report()
    }
}

impl InterfaceView {
    /// Attach to the scene's font-backed interface before it draws.
    pub fn attach(self, scene: &mut GameScene) {
        scene.set_menu(self.0);
    }
}

impl InterfaceFrame {
    /// Read the layout selected by the resident font publication.
    pub fn read(scene: &GameScene) -> Option<Self> {
        scene.submitted_menu().map(Self)
    }

    /// Return the input grid's extent, scale and safe area.
    pub fn viewport(&self) -> thrum_ui::Viewport {
        self.0.viewport
    }
}

/// Supply the drawable density and UIKit insets, in UIKit points.
pub fn set_output(scene: &mut GameScene, density: f64, insets: [f64; 4]) {
    scene.set_interface_output(density, insets);
}

/// All status node boxes in the current layout, for a held simulator capture.
pub fn status_bounds(scene: &GameScene) -> Option<serde_json::Value> {
    scene.status_bounds()
}
