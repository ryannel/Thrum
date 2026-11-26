//! Keep the inputs to a Metal stage key alive only as long as their creation handles.

use std::ops::Deref;

use super::DeviceSite;
#[cfg(target_os = "macos")]
use crate::capabilities::metallib::{InputKeys, KeyedInput};
#[cfg(target_os = "macos")]
use std::sync::Arc;

/// A shader or layout created by a device site. Clones share its optional Metal
/// stage-key metadata. Borrow it as a wgpu handle when building descriptors.
#[derive(Clone, Debug)]
pub struct PipelineInput<T>(Input<T>);

#[derive(Clone, Debug)]
enum Input<T> {
    Raw(T),
    #[cfg(target_os = "macos")]
    Keyed(Arc<KeyedInput<T>>),
}

impl<T> PipelineInput<T> {
    #[inline]
    pub(crate) fn raw(handle: T) -> Self {
        Self(Input::Raw(handle))
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn keyed(handle: T, keys: &InputKeys, key: blake3::Hash) -> Self {
        Self(Input::Keyed(keys.track(handle, key)))
    }
}

impl<T: Clone> PipelineInput<T> {
    /// Use the handle outside pipeline preparation, without its stage-key metadata.
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> T {
        match self.0 {
            Input::Raw(handle) => handle,
            #[cfg(target_os = "macos")]
            Input::Keyed(input) => input.handle.clone(),
        }
    }
}

impl<T> Deref for PipelineInput<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        match &self.0 {
            Input::Raw(handle) => handle,
            #[cfg(target_os = "macos")]
            Input::Keyed(input) => &input.handle,
        }
    }
}

impl DeviceSite {
    /// Create a bind group layout for use in a pipeline layout.
    pub fn create_bind_group_layout(
        &self,
        desc: &wgpu::BindGroupLayoutDescriptor<'_>,
    ) -> PipelineInput<wgpu::BindGroupLayout> {
        let layout = wgpu::Device::create_bind_group_layout(self, desc);
        #[cfg(target_os = "macos")]
        if let Some(preparation) = self.metal_preparation() {
            return preparation.bindings(layout, desc.entries);
        }
        PipelineInput::raw(layout)
    }

    /// Create a pipeline layout. Each explicit group must carry its descriptor
    /// metadata for the layout to have a Metal stage key.
    pub fn create_pipeline_layout(
        &self,
        desc: &wgpu::PipelineLayoutDescriptor<'_>,
    ) -> PipelineInput<wgpu::PipelineLayout> {
        let layout = wgpu::Device::create_pipeline_layout(self, desc);
        #[cfg(target_os = "macos")]
        if let Some(preparation) = self.metal_preparation() {
            return preparation.layout(layout, desc);
        }
        PipelineInput::raw(layout)
    }
}
