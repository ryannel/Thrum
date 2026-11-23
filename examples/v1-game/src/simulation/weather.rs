use super::{Rules, SimError, Simulation, WEATHER};
use std::sync::Arc;
use thrum_core::{
    Handle, StableId,
    sim::{
        ComponentKey, ComponentRegistry, ComponentSchemaId, GameRuntime, ReadPermit, WorldTxn,
        WritePermit,
    },
};

#[derive(Clone, PartialEq)]
pub(super) struct Weather(pub(super) Arc<thrum_cloud_model::State>);

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<ComponentKey, SimError> {
    let key = registry
        .register_with_snapshot::<Weather>(ComponentSchemaId(6), Clone::clone, |_| {
            size_of::<thrum_cloud_model::State>() + 2 * size_of::<usize>()
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
    ) -> Result<(), SimError> {
        let handle = self.handle.ok_or(SimError::InvalidConfig)?;
        let mut value = world
            .with(&self.read, handle, Clone::clone)
            .map_err(|_| SimError::InvalidHandle)?;
        let state = Arc::make_mut(&mut value.0);
        let time = state.time();
        let smooth = |unit: f64| {
            let t = unit.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        if state.shape_source(1) == Some(thrum_cloud_model::ShapeSource::Growth) {
            let growth = ((time - 12.0) / 120.0).clamp(0.0, 1.0);
            let forming = smooth(time / 18.0).max(1e-6);
            let thinning = 1.0 - smooth((time - 174.0) / 60.0);
            state
                .direct(1, forming * thinning, growth)
                .map_err(|_| SimError::InvalidConfig)?;
        }
        if state.shape_source(2).is_some() {
            let forming = smooth(time / 8.0).max(1e-6);
            let thinning = 1.0 - smooth((time - 22.0) / 65.0);
            state
                .direct(2, forming * thinning, 0.0)
                .map_err(|_| SimError::InvalidConfig)?;
        }
        state.step(tick, dt).map_err(|_| SimError::NonFinite)?;
        world
            .put(&self.write, handle, value)
            .map_err(|_| SimError::InvalidHandle)?;
        Ok(())
    }
}

impl Simulation {
    pub(crate) fn seed_weather(
        &mut self,
        state: Option<thrum_cloud_model::State>,
    ) -> Result<(), SimError> {
        if let Some(permits) = &mut self.rules.weather {
            let initial = state.ok_or(SimError::InvalidConfig)?;
            let handle = self
                .runtime
                .seed(StableId(u128::MAX), |writer| {
                    writer.add(super::Position([0.0; 3]))?;
                    writer.add(super::Health(1))?;
                    writer.add(Weather(Arc::new(initial)))?;
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
