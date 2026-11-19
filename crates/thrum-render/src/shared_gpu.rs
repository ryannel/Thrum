//! Shared mutable GPU state stays on the WebGPU thread in the browser.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc as Shared, Mutex as Cell};
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell as Cell, rc::Rc as Shared};

pub(crate) struct SharedGpu<T>(Shared<Cell<T>>);

impl<T> Clone for SharedGpu<T> {
    fn clone(&self) -> Self {
        Self(Shared::clone(&self.0))
    }
}

impl<T> SharedGpu<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Shared::new(Cell::new(value)))
    }

    pub(crate) fn write(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        #[cfg(target_arch = "wasm32")]
        {
            self.0.borrow_mut()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        }
    }
}
