use std::{collections::BTreeMap, sync::Arc, time::Duration};

use serde_json::{Value, json};
use thrum_app::{ClipboardAccess, KeyCode};
use thrum_ui::{Contact, Event, Held, InputLayout, Routed, Router, TextSource};

use super::{CARDS, CHOICE, COPY, HOST_TICKET, MenuFrame, MenuView, SUBMIT, Screen, View, policy};

const ARM_DELAY: Duration = Duration::from_millis(140);
const CONNECT_DELAY: Duration = Duration::from_millis(800);

pub(crate) struct Menu {
    router: Router<KeyCode>,
    view: View,
    mailbox: MenuView,
    layout: Option<Arc<InputLayout>>,
    viewport_scale: f64,
    armed_at: Duration,
    now: Duration,
    connects_at: Option<Duration>,
    closes_at: Option<Duration>,
    launch: Option<(String, TextSource)>,
    held: BTreeMap<Contact<KeyCode>, Option<[f64; 2]>>,
    proof: Option<Proof>,
    #[cfg(target_os = "macos")]
    frozen: bool,
    #[cfg(target_os = "macos")]
    pub(crate) script: Option<thrum_app::InterfaceScript>,
}

#[derive(Default)]
struct Proof {
    inputs: Vec<Value>,
    effects: Vec<Value>,
    clipboard: Vec<Value>,
    timing: Vec<Value>,
    awaiting: Vec<(u64, Arc<View>)>,
    latest: u64,
    activations: [u64; 4],
    position: Option<[f64; 3]>,
    submitted: Option<Arc<View>>,
}

impl Menu {
    pub(crate) fn new(proof: bool) -> Self {
        Self {
            router: Router::default(),
            view: View::default(),
            mailbox: MenuView::default(),
            layout: None,
            viewport_scale: 1.0,
            armed_at: Duration::ZERO,
            now: Duration::ZERO,
            connects_at: None,
            closes_at: None,
            launch: None,
            held: BTreeMap::new(),
            proof: proof.then(Proof::default),
            #[cfg(target_os = "macos")]
            frozen: false,
            #[cfg(target_os = "macos")]
            script: None,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn hold(&mut self, view: View) {
        self.view = view;
        self.frozen = true;
        self.mailbox.set(&self.view);
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn record(&mut self) {
        self.proof = Some(Proof::default());
    }

    pub(crate) fn mailbox(&self) -> MenuView {
        self.mailbox.clone()
    }

    pub(crate) fn launch(&mut self, ticket: String, source: TextSource, now: Duration) {
        self.screen(Screen::Joining, now);
        self.launch = Some((ticket, source));
    }

    pub(crate) fn screen(&mut self, screen: Screen, now: Duration) {
        let now = self.observe_time(now);
        let previous = self.view.screen.scope();
        let next = screen.scope();
        if previous != next {
            if let Some(previous) = previous {
                let routed = self.router.close(previous);
                self.receive(routed, now);
            }
            if let Some(next) = next {
                self.armed_at = if next == CHOICE { now + ARM_DELAY } else { now };
                let routed = self.router.open(next, policy(next), self.armed_at);
                self.receive(routed, now);
                self.effect(json!({"scope_opened": next}), now);
            }
        }
        self.view.screen = screen;
        self.closes_at = (screen == Screen::Joined).then(|| now + Duration::from_secs(12));
        if screen != Screen::Connecting {
            self.connects_at = None;
        }
        self.refresh(now);
    }

    pub(crate) fn tick(&mut self, now: Duration) {
        let now = self.observe_time(now);
        if self.connects_at.is_some_and(|deadline| now >= deadline) {
            let screen = if self.view.ticket.as_ref() == HOST_TICKET {
                Screen::Joined
            } else {
                Screen::Refused
            };
            self.screen(screen, now);
        }
        if self.closes_at.is_some_and(|deadline| now >= deadline) {
            self.screen(Screen::Closed, now);
        }
        self.refresh(now);
    }

    pub(crate) fn event(&mut self, event: Event<KeyCode>, now: Duration) {
        let now = self.observe_time(now);
        let before = self.mailbox.get();
        let kind = input_kind(&event);
        let event = points(event, self.viewport_scale);
        let start = self.proof.as_ref().map(|_| route_start());
        let routed = self.router.route(event, now);
        if let (Some(start), Some(proof)) = (start, self.proof.as_mut()) {
            let id = proof.timing.len();
            proof.timing.push(json!({
                "id": id, "kind": kind, "route_ms": route_elapsed(start),
                "event_ms": now.as_secs_f64() * 1000.0, "seen_through": proof.latest
            }));
        }
        self.receive(routed, now);
        self.refresh(now);
        let after = self.mailbox.get();
        if let Some(proof) = &mut self.proof
            && !Arc::ptr_eq(&before, &after)
        {
            let id = u64::try_from(proof.timing.len() - 1).expect("bounded event log");
            proof.awaiting.push((id, after));
        }
    }

    fn receive(&mut self, routed: Option<Routed<KeyCode>>, now: Duration) {
        match routed {
            Some(Routed::Interrupted) => {
                self.held.clear();
                self.effect(json!({"interrupted": true}), now);
            }
            Some(Routed::Resumed(held)) => {
                for Held { contact, position } in &held {
                    self.held.insert(*contact, *position);
                }
                self.effect(json!({"resumed": held.iter().map(|contact| format!("{contact:?}")).collect::<Vec<_>>() }), now);
            }
            Some(Routed::Game(event)) => self.game_event(event, now),
            Some(Routed::Activated(id)) if id == COPY => {
                self.router.request_copy(HOST_TICKET.to_owned());
                self.effect(json!({"copy": HOST_TICKET}), now);
            }
            Some(Routed::Activated(id)) if id == SUBMIT => {
                self.screen(Screen::Connecting, now);
                self.connects_at = Some(now + CONNECT_DELAY);
            }
            Some(Routed::Activated(id)) => {
                if let Some(index) = CARDS.iter().position(|value| *value == id) {
                    if let Some(proof) = &mut self.proof {
                        proof.activations[index] += 1;
                    }
                    self.effect(json!({"activated": id}), now);
                    self.screen(Screen::Play, now);
                }
            }
            Some(Routed::Cancelled(_)) => self.screen(Screen::Play, now),
            Some(Routed::Text(_, value, source)) => {
                self.effect(json!({"text": value, "source": format!("{source:?}")}), now);
                self.view.ticket = value.into();
            }
            None => {}
        }
    }

    fn game_event(&mut self, event: Event<KeyCode>, now: Duration) {
        if let Some(proof) = &mut self.proof {
            proof
                .inputs
                .push(json!({"input": format!("{event:?}"), "ms": now.as_secs_f64() * 1000.0}));
        }
        match event {
            Event::Press {
                contact, position, ..
            } => {
                self.held.insert(contact, position);
                match contact {
                    Contact::Key(KeyCode::KeyM) => self.screen(Screen::Choice, now),
                    Contact::Key(KeyCode::KeyJ) => self.screen(Screen::Joining, now),
                    Contact::Key(KeyCode::KeyH) => self.screen(Screen::Hosting, now),
                    Contact::Key(KeyCode::KeyE) => self.view.edit = !self.view.edit,
                    _ => {}
                }
            }
            Event::Release(contact, _) | Event::Cancel(contact) => {
                self.held.remove(&contact);
            }
            Event::Move(contact, position) => {
                if let Some(previous) = self.held.get_mut(&contact) {
                    *previous = Some(position);
                }
            }
            Event::Text(value, source) if self.view.screen == Screen::Play => {
                self.launch(value, source, now);
            }
            _ => {}
        }
    }

    pub(crate) fn axes(&self) -> [f32; 3] {
        let key = |code| self.held.contains_key(&Contact::Key(code));
        let [left, right, down, up] = [
            key(KeyCode::KeyA) || key(KeyCode::ArrowLeft),
            key(KeyCode::KeyD) || key(KeyCode::ArrowRight),
            key(KeyCode::KeyS) || key(KeyCode::ArrowDown),
            key(KeyCode::KeyW) || key(KeyCode::ArrowUp),
        ];
        let stick = self
            .held
            .iter()
            .find_map(|(contact, position)| {
                if let (Contact::Touch(_), Some([x, _])) = (contact, position)
                    && *x <= 120.0
                {
                    return Some(if *x >= 56.0 { 1.0 } else { -1.0 });
                }
                None
            })
            .unwrap_or_default();
        [
            (stick + f32::from(u8::from(right)) - f32::from(u8::from(left))).clamp(-1.0, 1.0),
            f32::from(u8::from(up)) - f32::from(u8::from(down)),
            0.0,
        ]
    }

    pub(crate) fn submitted(
        &mut self,
        frame: &MenuFrame,
        index: u64,
        player: Option<[f64; 3]>,
        now: Duration,
    ) {
        let now = self.observe_time(now);
        if let Some(proof) = &mut self.proof {
            proof.latest = index;
            proof.position = player;
            proof.submitted = Some(Arc::clone(&frame.view));
            proof.awaiting.retain(|(id, view)| {
                if !Arc::ptr_eq(view, &frame.view) {
                    return true;
                }
                let row = &mut proof.timing[usize::try_from(*id).expect("bounded event log")];
                row["first_submission"] = json!(index);
                row["frames"] =
                    json!(index - row["seen_through"].as_u64().expect("submission count"));
                false
            });
        }
        if !self
            .layout
            .as_ref()
            .is_some_and(|shown| Arc::ptr_eq(shown, &frame.input))
        {
            self.viewport_scale = f64::from(frame.viewport.scale);
            self.router.submitted((*frame.input).clone());
            self.layout = Some(Arc::clone(&frame.input));
        }
        if frame.view.screen == Screen::Joining
            && let Some((value, source)) = self.launch.take()
        {
            self.event(Event::Text(value, source), now);
        }
        self.refresh(now);
    }

    pub(crate) fn take_copy(&mut self) -> Option<String> {
        self.router.take_copy()
    }

    pub(crate) fn clipboard(&mut self, access: ClipboardAccess, allowed: bool) {
        if let Some(proof) = &mut self.proof {
            proof
                .clipboard
                .push(json!({"access": format!("{access:?}"), "allowed": allowed}));
        }
    }

    fn effect(&mut self, effect: Value, now: Duration) {
        if let Some(proof) = &mut self.proof {
            let mut row = json!({"ms": now.as_secs_f64() * 1000.0});
            row["effect"] = effect;
            proof.effects.push(row);
        }
    }

    fn observe_time(&mut self, now: Duration) -> Duration {
        // A browser animation timestamp can precede the preceding DOM event.
        self.now = self.now.max(now);
        self.now
    }

    fn refresh(&mut self, now: Duration) {
        #[cfg(target_os = "macos")]
        if self.frozen {
            return;
        }
        self.view.disarmed = self.view.screen == Screen::Choice && now < self.armed_at;
        self.view.cards = CARDS.map(|id| self.router.button_state(id));
        self.view.join_button = self
            .router
            .button_state(if self.view.screen == Screen::Hosting {
                COPY
            } else {
                SUBMIT
            });
        self.mailbox.set(&self.view);
    }

    pub(crate) fn report(&self) -> Value {
        let controls = [CARDS.as_slice(), &[COPY, SUBMIT]].concat();
        let boxes = controls
            .into_iter()
            .filter_map(|id| {
                let bounds = self.layout.as_ref()?.bounds(id)?;
                Some(json!([id, bounds.origin, bounds.size]))
            })
            .collect::<Vec<_>>();
        let proof = self.proof.as_ref();
        json!({
            "screen": format!("{:?}", self.view.screen),
            "drawn": proof.and_then(|proof| proof.submitted.as_ref()).map(|view| format!("{:?}", view.screen)),
            "focus": self.router.focus(),
            "disarmed": self.view.disarmed,
            "boxes": boxes,
            "scale": self.viewport_scale,
            "held": format!("{:?}", self.held),
            "ticket": self.view.ticket.as_ref(),
            "refused": self.router.refused_drops(),
            "submitted": proof.map(|proof| proof.latest),
            "position": proof.and_then(|proof| proof.position),
            "activations": proof.map(|proof| proof.activations),
            "inputs": proof.map(|proof| &proof.inputs),
            "effects": proof.map(|proof| &proof.effects),
            "clipboard": proof.map(|proof| &proof.clipboard),
            "timing": proof.map(|proof| &proof.timing),
            "route_clock": if cfg!(target_os = "macos") { "thread CPU time" } else { "Performance.now" },
        })
    }
}

fn input_kind(event: &Event<KeyCode>) -> &'static str {
    match event {
        Event::Press {
            contact: Contact::Key(KeyCode::KeyE),
            ..
        } => "presentation edit",
        Event::Press {
            contact: Contact::Key(_),
            ..
        } => "key press",
        Event::Press {
            contact: Contact::Pointer(_),
            ..
        } => "pointer press",
        Event::Press {
            contact: Contact::Touch(_),
            ..
        } => "touch press",
        _ => "other",
    }
}

fn points(event: Event<KeyCode>, scale: f64) -> Event<KeyCode> {
    let convert = |point: [f64; 2]| point.map(|axis| axis / scale);
    match event {
        Event::Press {
            contact,
            position,
            intent,
        } => Event::Press {
            contact,
            position: position.map(convert),
            intent,
        },
        Event::Release(contact, point) => Event::Release(contact, point.map(convert)),
        Event::Move(contact, point) => Event::Move(contact, convert(point)),
        Event::Hover(point) => Event::Hover(convert(point)),
        event => event,
    }
}

#[cfg(target_arch = "wasm32")]
fn route_start() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map_or(0.0, |clock| clock.now())
}

#[cfg(target_arch = "wasm32")]
fn route_elapsed(start: f64) -> f64 {
    (route_start() - start).max(0.0)
}

#[cfg(target_os = "macos")]
fn route_start() -> cpu_time::ThreadTime {
    cpu_time::ThreadTime::now()
}

#[cfg(target_os = "macos")]
fn route_elapsed(start: cpu_time::ThreadTime) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
