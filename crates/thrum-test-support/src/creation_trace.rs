//! Gross GPU object creations, including short-lived objects.

use std::sync::{Arc, Mutex};
use tracing::{Dispatch, Subscriber, span};
use tracing_subscriber::{Layer, layer::Context, prelude::*};

/// Counts GPU object creation events directly from wgpu_core spans by thread.
#[derive(Clone, Default)]
pub struct Creations(Arc<Mutex<Vec<(std::thread::ThreadId, &'static str)>>>);

impl<S: Subscriber> Layer<S> for Creations {
    fn on_new_span(&self, attributes: &span::Attributes<'_>, _: &span::Id, _: Context<'_, S>) {
        let metadata = attributes.metadata();
        if metadata.target().starts_with("wgpu_core::")
            && matches!(
                metadata.name(),
                "Device::create_texture"
                    | "Device::create_texture_from_hal"
                    | "Texture::create_view"
                    | "Device::create_bind_group"
                    | "Device::create_general_render_pipeline"
                    | "Device::create_compute_pipeline"
            )
        {
            self.0
                .lock()
                .expect("creation event")
                .push((std::thread::current().id(), metadata.name()));
        }
    }
}

impl Creations {
    /// Creation event names in recorded order for this thread.
    ///
    /// # Panics
    /// Panics if a test has poisoned the event recorder.
    pub fn on(&self, thread: std::thread::ThreadId) -> Vec<&'static str> {
        self.0
            .lock()
            .expect("creation events")
            .iter()
            .filter_map(|&(creator, kind)| (creator == thread).then_some(kind))
            .collect()
    }

    /// Dispatch to install on the render and worker threads under observation.
    pub fn dispatch(&self) -> Dispatch {
        Dispatch::new(tracing_subscriber::registry().with(self.clone()))
    }
}

/// Install a creation dispatch for the duration of a call on this thread.
pub use tracing::dispatcher::with_default;
