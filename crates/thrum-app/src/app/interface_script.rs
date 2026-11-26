//! Drives the window's input handlers from a bounded fixture after an actual submission.

use std::{collections::VecDeque, path::PathBuf};

use serde_json::{Value, json};
use thrum_ui::{Contact, Event};
use winit::{
    event::{ElementState, MouseButton},
    keyboard::ModifiersState,
};

use super::{App, RenderJob, WindowGame};
use crate::KeyCode;

/// File paths for an unattended input qualification run in the ordinary window.
#[derive(Clone)]
pub struct InterfaceScript {
    /// Read the scripted actions from this JSON file.
    pub input: PathBuf,
    /// Write the game's record of those actions here.
    /// A checkpoint action also writes the steps so far with extension `checkpoint.json`.
    pub output: PathBuf,
}

pub(super) struct Script {
    steps: VecDeque<Value>,
    log: Vec<Value>,
    last: usize,
    output: PathBuf,
}

impl Script {
    pub(super) fn new(config: &InterfaceScript) -> Result<Self, String> {
        let bytes = std::fs::read(&config.input).map_err(|error| error.to_string())?;
        let steps: VecDeque<Value> =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if steps.is_empty() || steps.len() > 256 {
            return Err("interface script requires 1..=256 actions".into());
        }
        Ok(Self {
            steps,
            log: Vec::new(),
            last: 0,
            output: config.output.clone(),
        })
    }
}

impl<G: WindowGame + 'static> App<G> {
    pub(super) fn step_interface_script(&mut self) -> Result<(), String> {
        let Some(mut script) = self.interface_script.take() else {
            return Ok(());
        };
        let result = self.run_interface_step(&mut script);
        self.interface_script = Some(script);
        result
    }

    fn run_interface_step(&mut self, script: &mut Script) -> Result<(), String> {
        let Some(step) = script.steps.front() else {
            return Ok(());
        };
        let after = usize::try_from(step["after"].as_u64().unwrap_or(1))
            .map_err(|error| error.to_string())?;
        if self.presented < script.last.saturating_add(after) {
            return Ok(());
        }
        let report = self
            .game
            .interface_witness()
            .ok_or("missing interface witness")?;
        let witness: Value = serde_json::from_str(&report).map_err(|error| error.to_string())?;
        if witness["drawn"].is_null() {
            return Ok(());
        }
        let event_at = self.origin.elapsed().as_secs_f64() * 1000.0;
        let action = step["action"]
            .as_str()
            .ok_or("interface step needs an action")?;
        let checkpoint = action == "checkpoint";
        let point = self.perform_interface_step(action, step, &witness)?;
        script.last = self.presented;
        script.log.push(json!({
            "step": script.steps.pop_front(), "time_ms": event_at, "presented": self.presented,
            "device": self.recovery.active, "before": witness, "position_px": point,
        }));
        if checkpoint {
            let checkpoint = json!({"steps": script.log});
            std::fs::write(
                script.output.with_extension("checkpoint.json"),
                checkpoint.to_string(),
            )
            .map_err(|error| error.to_string())?;
        }
        if script.steps.is_empty() {
            let complete = json!({"steps": script.log, "final": self.game.interface_witness()});
            std::fs::write(&script.output, complete.to_string())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn perform_interface_step(
        &mut self,
        action: &str,
        step: &Value,
        witness: &Value,
    ) -> Result<Option<[f64; 2]>, String> {
        let state = if step["down"].as_bool().unwrap_or(true) {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
        match action {
            "key" => {
                let code = step["code"]
                    .as_str()
                    .and_then(KeyCode::from_code)
                    .ok_or("unknown scripted physical key")?;
                self.interface_input.modifiers.set(
                    ModifiersState::SUPER,
                    step["command"].as_bool().unwrap_or(false),
                );
                self.interface_key(code, state, step["repeat"].as_bool().unwrap_or(false), true)?;
            }
            "cursor" => {
                let point = scripted_point(step, witness)?;
                self.interface_cursor(point)?;
                return Ok(Some(point));
            }
            "button" => self.interface_mouse(MouseButton::Left, state)?,
            "touch" => {
                let point = scripted_point(step, witness)?;
                let contact = Contact::Touch(step["id"].as_u64().ok_or("touch needs an id")?);
                self.interface_event(if state == ElementState::Pressed {
                    Event::Press {
                        contact,
                        position: Some(point),
                        intent: None,
                    }
                } else {
                    Event::Release(contact, Some(point))
                })?;
                return Ok(Some(point));
            }
            "recover" => {
                self.render
                    .as_ref()
                    .ok_or("render worker missing")?
                    .try_send(RenderJob::LoseForInterfaceProof)
                    .map_err(|_| "render worker refused the scripted device loss")?;
                self.render_in_flight = Some(0);
            }
            "observe" | "checkpoint" => {}
            _ => return Err(format!("unknown interface step {action}")),
        }
        Ok(None)
    }
}

fn pair(value: &Value) -> Option<[f64; 2]> {
    Some([value[0].as_f64()?, value[1].as_f64()?])
}

fn scripted_point(step: &Value, witness: &Value) -> Result<[f64; 2], String> {
    if let Some(point) = pair(&step["point"]) {
        return Ok(point);
    }
    let id = step["control"]
        .as_u64()
        .ok_or("script needs an output pixel or control")?;
    let entry = witness["boxes"]
        .as_array()
        .and_then(|boxes| boxes.iter().find(|entry| entry[0].as_u64() == Some(id)))
        .ok_or_else(|| format!("script control {id} has no submitted box"))?;
    let origin = pair(&entry[1]).ok_or("missing control origin")?;
    let size = pair(&entry[2]).ok_or("missing control size")?;
    let scale = witness["scale"].as_f64().ok_or("missing submitted scale")?;
    Ok(std::array::from_fn(|axis| {
        (size[axis].mul_add(0.5, origin[axis]) * scale).round()
    }))
}
