//! Pure controller rule shared by the authoritative fixture stages and a
//! speculative client branch. This remains the bounded A1 game fixture rule.

use std::num::NonZeroU32;

use super::{
    FrameError, SimAction,
    world::{ControllerState, Position, SimConfig, Velocity},
};

/// One already authored controller input, or a tick with no new input.
#[derive(Clone, Copy, Debug)]
pub enum ControllerInput<'a> {
    /// Hold the previous continuous axes; never repeat actions.
    Hold,
    /// Replace held axes and apply these one-shot actions once.
    Frame {
        /// Finite normalized movement axes.
        axes: [f32; 3],
        /// Ordered fixture actions.
        actions: &'a [SimAction],
    },
}

/// Refusal from the pure controller replay boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerStepError {
    /// The authored axes or action payload failed fixture admission.
    InvalidInput(FrameError),
    /// A state, derived velocity or integrated position was nonfinite.
    NonFinite,
}

/// Immutable controller recipe copied from the authority at session start.
#[derive(Clone, Copy, Debug)]
pub struct ControllerRule {
    tick_hz: NonZeroU32,
    movement_speed: f64,
    actions_per_frame: usize,
}

impl ControllerRule {
    pub(super) fn from_config(config: SimConfig) -> Self {
        Self {
            tick_hz: config.tick_hz,
            movement_speed: config.movement_speed,
            actions_per_frame: config.actions_per_frame,
        }
    }

    /// Advance one complete speculative controller value through the same
    /// input and integration calculations used by the authoritative stages.
    /// This does not admit gameplay or publish world state.
    ///
    /// # Errors
    /// Refuses invalid axes/actions or any nonfinite state/result.
    pub fn advance(
        self,
        state: ControllerState,
        input: ControllerInput<'_>,
    ) -> Result<ControllerState, ControllerStepError> {
        let (axes, actions) = match input {
            ControllerInput::Hold => (state.held_axes, &[][..]),
            ControllerInput::Frame { axes, actions } => (axes, actions),
        };
        if axes
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > 1.0)
        {
            return Err(ControllerStepError::InvalidInput(FrameError::InvalidAxis));
        }
        if actions.len() > self.actions_per_frame {
            return Err(ControllerStepError::InvalidInput(
                FrameError::TooManyActions,
            ));
        }
        if actions
            .iter()
            .any(|action| matches!(action, SimAction::Impulse(values) if !finite(*values)))
        {
            return Err(ControllerStepError::InvalidInput(FrameError::InvalidAction));
        }
        if !finite(state.position.0) || !finite(state.velocity.0) {
            return Err(ControllerStepError::NonFinite);
        }
        let (velocity, health, _) =
            resolve_control(axes, actions, state.health, self.movement_speed);
        let position = integrate_position(
            state.position,
            velocity,
            1.0 / f64::from(self.tick_hz.get()),
        );
        if !finite(velocity.0) || !finite(position.0) {
            return Err(ControllerStepError::NonFinite);
        }
        Ok(ControllerState {
            position,
            velocity,
            health,
            held_axes: axes,
            ..state
        })
    }
}

pub(super) fn resolve_control(
    axes: [f32; 3],
    actions: &[SimAction],
    initial_health: u16,
    movement_speed: f64,
) -> (Velocity, u16, bool) {
    let mut velocity = Velocity(axes.map(|axis| f64::from(axis) * movement_speed));
    let mut health = initial_health;
    let mut damaged = false;
    for action in actions {
        match action {
            SimAction::Impulse(impulse) => {
                for (value, delta) in velocity.0.iter_mut().zip(impulse) {
                    *value += delta;
                }
            }
            SimAction::Damage(damage) => {
                health = health.saturating_sub(*damage);
                damaged = true;
            }
        }
    }
    (velocity, health, damaged)
}

pub(super) fn integrate_position(mut position: Position, velocity: Velocity, dt: f64) -> Position {
    for (value, speed) in position.0.iter_mut().zip(velocity.0) {
        *value += speed * dt;
    }
    position
}

fn finite(values: [f64; 3]) -> bool {
    values.into_iter().all(f64::is_finite)
}
