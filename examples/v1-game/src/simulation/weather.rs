use super::{Rules, SimError, Simulation, WEATHER};
#[cfg(feature = "proving")]
use crate::clouds::WeatherSequence;
use std::sync::Arc;
use thrum_core::{
    Handle, StableId,
    sim::{
        ComponentKey, ComponentRegistry, ComponentSchemaId, GameRuntime, ReadPermit, WorldTxn,
        WritePermit,
    },
};

#[derive(Clone, PartialEq)]
pub(crate) struct Weather {
    pub(super) state: Arc<thrum_cloud_model::State>,
    demo: Option<crate::clouds::PlacementDemo>,
    ground: Option<GroundDay>,
    #[cfg(feature = "proving")]
    sequence: Option<(WeatherSequence, thrum_cloud_model::Parameters)>,
}

impl Weather {
    pub(crate) fn new(
        state: thrum_cloud_model::State,
        demo: Option<crate::clouds::PlacementDemo>,
    ) -> Self {
        Self {
            state: Arc::new(state),
            demo,
            ground: None,
            #[cfg(feature = "proving")]
            sequence: None,
        }
    }

    pub(crate) fn with_ground(mut self, fog: Arc<thrum_cloud_model::Ground>) -> Self {
        self.ground = Some(GroundDay {
            fog,
            solar: SolarTime::default(),
        });
        self
    }

    pub(crate) fn solar(&self) -> Option<SolarTime> {
        self.ground.as_ref().map(|ground| ground.solar)
    }

    #[cfg(feature = "proving")]
    pub(crate) fn with_sequence(
        mut self,
        sequence: WeatherSequence,
        initial: thrum_cloud_model::Parameters,
    ) -> Self {
        if sequence != WeatherSequence::Held {
            self.sequence = Some((sequence, initial));
        }
        self
    }
}

#[derive(Clone, PartialEq)]
struct GroundDay {
    fog: Arc<thrum_cloud_model::Ground>,
    solar: SolarTime,
}

/// The endpoints of one committed solar tick, independent of advection.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct SolarTime {
    previous: f64,
    advance: f64,
}

impl Default for SolarTime {
    fn default() -> Self {
        Self {
            previous: 0.5,
            advance: 0.0,
        }
    }
}

impl SolarTime {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the normalized solar phase fits the Sun sampler"
    )]
    pub(crate) fn sample(self, alpha: f32) -> f32 {
        self.at(f64::from(alpha)) as f32
    }

    fn at(self, alpha: f64) -> f64 {
        self.advance.mul_add(alpha, self.previous).rem_euclid(1.0)
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<ComponentKey, SimError> {
    let key = registry
        .register_with_snapshot::<Weather>(ComponentSchemaId(6), Clone::clone, |_| {
            size_of::<thrum_cloud_model::State>() + size_of::<Weather>() + size_of::<usize>()
        })
        .map_err(|_| SimError::InvalidConfig)?;
    registry
        .select_publication::<Weather>(false)
        .map_err(|_| SimError::InvalidConfig)?;
    Ok(key)
}

pub(super) struct Permits {
    pub(super) handle: Option<Handle>,
    read: ReadPermit<Weather>,
    write: WritePermit<Weather>,
}

impl Permits {
    pub(super) fn new(runtime: &GameRuntime<Rules>) -> Result<Self, SimError> {
        Ok(Self {
            handle: None,
            read: runtime.read_permit(WEATHER).map_err(super::map_runtime)?,
            write: runtime.write_permit(WEATHER).map_err(super::map_runtime)?,
        })
    }
    pub(super) fn step(
        &self,
        world: &mut WorldTxn<'_>,
        tick: u64,
        dt: f64,
        commands: &[super::Command],
    ) -> Result<(), SimError> {
        let handle = self.handle.ok_or(SimError::InvalidConfig)?;
        let mut value = world
            .with(&self.read, handle, Clone::clone)
            .map_err(|_| SimError::InvalidHandle)?;
        let state = Arc::make_mut(&mut value.state);
        let mut solar_edit = None;
        for command in commands {
            match *command {
                super::Command::SetWeather(target) => {
                    state
                        .set_target(target)
                        .map_err(|_| SimError::InvalidConfig)?;
                    #[cfg(feature = "proving")]
                    {
                        value.sequence = None;
                    }
                }
                super::Command::SetGroundWeather(target) => {
                    if value.ground.is_none() {
                        return Err(SimError::InvalidConfig);
                    }
                    target
                        .apply(state.ground_mut())
                        .map_err(|_| SimError::InvalidConfig)?;
                }
                super::Command::SetWeatherSolarTime(fraction) => solar_edit = Some(fraction),
                _ => {}
            }
        }
        #[cfg(feature = "proving")]
        if let Some((sequence, initial)) = value.sequence {
            let seconds = tick as f64 * dt;
            if sequence == WeatherSequence::MistSweep {
                let t = (1.0 - (seconds / 60.0 - 1.0).abs()).clamp(0.0, 1.0);
                let amount = t * t * (3.0 - 2.0 * t);
                state
                    .ground_mut()
                    .set_targets(&[amount, 0.0], 0.0, 244_000.0)
                    .map_err(|_| SimError::InvalidConfig)?;
            }
            state
                .set_target(sequence.target(initial, seconds))
                .map_err(|_| SimError::InvalidConfig)?;
        }
        if let Some(demo) = value.demo {
            demo.step(state).map_err(|_| SimError::InvalidConfig)?;
        }
        state.step(tick, dt).map_err(|_| SimError::NonFinite)?;
        if let Some(ground) = &mut value.ground {
            // At 1× the valley's authored day lasts five minutes. The cooling
            // model consumes hours of solar time while wind consumes game seconds.
            let radiative = state.ground().radiative_day();
            let rate = if radiative {
                state.parameters().time_scale
            } else {
                1.0
            };
            let advance = dt * rate / crate::clouds::ground::SOLAR_DAY_SECONDS;
            ground.solar = SolarTime {
                previous: solar_edit.map_or_else(|| ground.solar.at(1.0), f64::from),
                advance: if solar_edit.is_some() { 0.0 } else { advance },
            };
            if radiative {
                let sun = crate::clouds::ground::sun(ground.solar.sample(1.0), tick, 1.0);
                state
                    .advance_ground_cooling(
                        &ground.fog,
                        advance * 86_400.0,
                        f64::from(sun.toward_sun[1].asin().to_degrees()),
                    )
                    .map_err(|_| SimError::InvalidConfig)?;
            }
        }
        world
            .put(&self.write, handle, value)
            .map_err(|_| SimError::InvalidHandle)?;
        Ok(())
    }
}

impl Simulation {
    pub(crate) fn seed_weather(&mut self, state: Option<Weather>) -> Result<(), SimError> {
        if let Some(permits) = &mut self.rules.weather {
            let initial = state.ok_or(SimError::InvalidConfig)?;
            let handle = self
                .runtime
                .seed(StableId(u128::MAX), |writer| {
                    writer.add(super::Position([0.0; 3]))?;
                    writer.add(super::Health(1))?;
                    writer.add(initial)?;
                    Ok(())
                })
                .map_err(super::map_runtime)?;
            permits.handle = Some(handle);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
