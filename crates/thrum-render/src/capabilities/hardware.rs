//! Named experimental hardware admission at device creation.

use super::DeviceCapabilities;
use std::fmt;

/// Experimental hardware that a launch can request explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareFeature {
    /// Object and mesh shaders.
    MeshStages,
    /// Ray queries in shaders.
    RayQuery,
    /// Direct loading of Metal shader libraries during pipeline preparation.
    #[cfg(target_os = "macos")]
    MetalShaderPassthrough,
}

impl HardwareFeature {
    /// The launch spelling of the feature.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MeshStages => "mesh-stages",
            Self::RayQuery => "ray-query",
            #[cfg(target_os = "macos")]
            Self::MetalShaderPassthrough => "metal-shader-passthrough",
        }
    }

    fn flag(self) -> wgpu::Features {
        match self {
            Self::MeshStages => wgpu::Features::EXPERIMENTAL_MESH_SHADER,
            Self::RayQuery => wgpu::Features::EXPERIMENTAL_RAY_QUERY,
            #[cfg(target_os = "macos")]
            Self::MetalShaderPassthrough => wgpu::Features::PASSTHROUGH_SHADERS,
        }
    }

    fn limits(self, mut base: wgpu::Limits, offered: &wgpu::Limits) -> wgpu::Limits {
        match self {
            Self::MeshStages => {
                base.max_storage_buffers_per_shader_stage =
                    base.max_storage_buffers_per_shader_stage.max(6);
                base.max_bind_groups = base.max_bind_groups.max(5);
                if base.max_task_workgroup_total_count == 0 {
                    base.max_task_workgroup_total_count =
                        offered.max_task_workgroup_total_count.max(1);
                }
                if base.max_task_workgroups_per_dimension == 0 {
                    base.max_task_workgroups_per_dimension =
                        offered.max_task_workgroups_per_dimension.max(1);
                }
                base.max_mesh_workgroup_total_count = base.max_mesh_workgroup_total_count.max(64);
                base.max_mesh_workgroups_per_dimension =
                    base.max_mesh_workgroups_per_dimension.max(64);
                base.max_task_invocations_per_workgroup =
                    base.max_task_invocations_per_workgroup.max(1);
                base.max_task_invocations_per_dimension =
                    base.max_task_invocations_per_dimension.max(1);
                base.max_mesh_invocations_per_workgroup =
                    base.max_mesh_invocations_per_workgroup.max(1);
                base.max_mesh_invocations_per_dimension =
                    base.max_mesh_invocations_per_dimension.max(1);
                base.max_task_payload_size = base.max_task_payload_size.max(256);
                base.max_mesh_output_vertices = base.max_mesh_output_vertices.max(64);
                base.max_mesh_output_primitives = base.max_mesh_output_primitives.max(124);
            }
            #[cfg(target_os = "macos")]
            Self::MetalShaderPassthrough => {}
            Self::RayQuery => {
                let minimum =
                    wgpu::Limits::default().using_minimum_supported_acceleration_structure_values();
                base.max_blas_geometry_count = base
                    .max_blas_geometry_count
                    .max(minimum.max_blas_geometry_count);
                base.max_tlas_instance_count = base
                    .max_tlas_instance_count
                    .max(minimum.max_tlas_instance_count);
                base.max_blas_primitive_count = base
                    .max_blas_primitive_count
                    .max(minimum.max_blas_primitive_count);
                base.max_acceleration_structures_per_shader_stage = base
                    .max_acceleration_structures_per_shader_stage
                    .max(minimum.max_acceleration_structures_per_shader_stage);
                base.max_buffers_and_acceleration_structures_per_shader_stage = base
                    .max_buffers_and_acceleration_structures_per_shader_stage
                    .max(minimum.max_buffers_and_acceleration_structures_per_shader_stage);
            }
        }
        base
    }
}

/// One feature the launch wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareRequest {
    /// Requested capability.
    pub feature: HardwareFeature,
    /// Refuse device creation if this capability is unavailable.
    pub required: bool,
}

impl HardwareRequest {
    /// Parse the comma-separated launch list. A trailing `?` marks a preference.
    ///
    /// # Errors
    /// Names an unknown or repeated feature.
    pub fn parse(value: &str) -> Result<Vec<Self>, HardwareLaunchError> {
        let mut requests: Vec<Self> = Vec::new();
        if value.trim().is_empty() {
            return Ok(requests);
        }
        for part in value.split(',').map(str::trim) {
            let (name, required) = part
                .strip_suffix('?')
                .map_or((part, true), |name| (name, false));
            let feature = match name {
                "mesh-stages" => HardwareFeature::MeshStages,
                "ray-query" => HardwareFeature::RayQuery,
                #[cfg(target_os = "macos")]
                "metal-shader-passthrough" => HardwareFeature::MetalShaderPassthrough,
                _ => return Err(HardwareLaunchError::Unknown(name.to_owned())),
            };
            if requests.iter().any(|request| request.feature == feature) {
                return Err(HardwareLaunchError::Repeated(feature));
            }
            requests.push(Self { feature, required });
        }
        Ok(requests)
    }
}

/// A launch's hardware request could not be admitted.
#[derive(Debug)]
pub enum HardwareLaunchError {
    /// The restriction requested more than the selected profile.
    InvalidRestriction,
    /// A name is unknown to the registry.
    Unknown(String),
    /// A feature appears more than once in the list.
    Repeated(HardwareFeature),
    /// No adapter matched the host's request.
    Adapter(wgpu::RequestAdapterError),
    /// The adapter cannot admit required hardware.
    Required {
        /// Feature that must be enabled.
        feature: HardwareFeature,
        /// Why the adapter could not enable it.
        reason: String,
    },
    /// The adapter failed to make a device from the final request.
    Device {
        /// Required feature names on that request.
        required: Vec<HardwareFeature>,
        /// Backend failure.
        source: wgpu::RequestDeviceError,
    },
}

impl fmt::Display for HardwareLaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRestriction => {
                f.write_str("device restriction adds a feature or raises a limit")
            }
            Self::Unknown(name) => write!(f, "unknown hardware feature {name:?}"),
            Self::Repeated(feature) => write!(f, "repeated hardware feature {}", feature.name()),
            Self::Adapter(error) => write!(f, "adapter: {error}"),
            Self::Required { feature, reason } => {
                write!(f, "required {}: {reason}", feature.name())
            }
            Self::Device { required, source } => {
                f.write_str("device")?;
                if !required.is_empty() {
                    let names: Vec<_> = required.iter().map(|feature| feature.name()).collect();
                    write!(f, " with required {}", names.join(", "))?;
                }
                write!(f, ": {source}")
            }
        }
    }
}

impl std::error::Error for HardwareLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Adapter(error) => Some(error),
            Self::Device { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The generation's answer to one launch request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareSelection {
    /// What the launch asked for.
    pub request: HardwareRequest,
    /// Why the feature was declined, or none when enabled.
    pub declined: Option<String>,
}

fn plan(
    mut descriptor: wgpu::DeviceDescriptor<'static>,
    requests: &[HardwareRequest],
    features: wgpu::Features,
    limits: &wgpu::Limits,
    retry: Option<&str>,
) -> Result<(wgpu::DeviceDescriptor<'static>, Vec<HardwareSelection>), HardwareLaunchError> {
    let mut selections = Vec::new();
    for &request in requests {
        let feature = request.feature;
        let needed = feature.limits(descriptor.required_limits.clone(), limits);
        let declined = if !features.contains(feature.flag()) {
            Some("adapter does not offer the feature".into())
        } else if !needed.check_limits(limits) {
            Some("adapter cannot grant the needed limits".into())
        } else if !request.required
            && let Some(reason) = retry
        {
            Some(reason.to_owned())
        } else {
            descriptor.required_features |= feature.flag();
            descriptor.required_limits = needed;
            descriptor.experimental_features = experimental_features();
            None
        };
        if request.required
            && let Some(reason) = declined
        {
            return Err(HardwareLaunchError::Required { feature, reason });
        }
        selections.push(HardwareSelection { request, declined });
    }
    Ok((descriptor, selections))
}

#[expect(
    unsafe_code,
    reason = "explicit Track 5 launch opt-in requires wgpu's experimental token"
)]
fn experimental_features() -> wgpu::ExperimentalFeatures {
    // SAFETY: Only the explicitly requested, adapter-admitted Track 5 features reach this
    // branch. Their consumers validate shader and resource limits on the resulting device.
    unsafe { wgpu::ExperimentalFeatures::enabled() }
}

impl DeviceCapabilities {
    pub(super) async fn open_planned_generation(
        mut acquire_adapter: impl AsyncFnMut() -> Result<wgpu::Adapter, wgpu::RequestAdapterError>,
        generation: u64,
        requests: &[HardwareRequest],
        descriptor: impl Fn(
            &wgpu::Adapter,
        ) -> Result<
            (
                wgpu::DeviceDescriptor<'static>,
                (wgpu::Features, wgpu::Limits),
            ),
            HardwareLaunchError,
        >,
        adjust: impl Fn(
            &wgpu::Adapter,
            &mut wgpu::DeviceDescriptor<'static>,
        ) -> Result<(), HardwareLaunchError>,
    ) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue, Self), HardwareLaunchError> {
        for (index, request) in requests.iter().enumerate() {
            if requests[..index]
                .iter()
                .any(|previous| previous.feature == request.feature)
            {
                return Err(HardwareLaunchError::Repeated(request.feature));
            }
        }
        let mut retry = None;
        loop {
            let adapter = acquire_adapter()
                .await
                .map_err(HardwareLaunchError::Adapter)?;
            let (descriptor, (features, limits)) = descriptor(&adapter)?;
            let (mut request, selections) = plan(
                descriptor,
                requests,
                features & adapter.features(),
                &limits,
                retry.as_deref(),
            )?;
            adjust(&adapter, &mut request)?;
            match adapter.request_device(&request).await {
                Ok((device, queue)) => {
                    let mut capabilities = Self::capture(&adapter, &device, generation);
                    capabilities.hardware = selections;
                    return Ok((adapter, device, queue, capabilities));
                }
                Err(error)
                    if retry.is_none()
                        && selections
                            .iter()
                            .any(|s| !s.request.required && s.declined.is_none()) =>
                {
                    retry = Some(format!(
                        "device request with preferred hardware failed: {error}"
                    ));
                }
                Err(source) => {
                    let required = requests
                        .iter()
                        .filter(|r| r.required)
                        .map(|r| r.feature)
                        .collect();
                    return Err(HardwareLaunchError::Device { required, source });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_and_preferred_missing_features_or_limits() {
        let limits = wgpu::Limits::default();
        let cases = [
            (wgpu::Features::empty(), "does not offer"),
            (wgpu::Features::EXPERIMENTAL_MESH_SHADER, "cannot grant"),
        ];
        for (features, reason) in cases {
            let required = HardwareRequest::parse("mesh-stages").expect("required");
            let error = plan(
                wgpu::DeviceDescriptor::default(),
                &required,
                features,
                &limits,
                None,
            )
            .expect_err("required request refused")
            .to_string();
            assert!(
                error.contains("required mesh-stages") && error.contains(reason),
                "{error}"
            );
            let preferred = HardwareRequest::parse("mesh-stages?").expect("preferred");
            let (descriptor, selections) = plan(
                wgpu::DeviceDescriptor::default(),
                &preferred,
                features,
                &limits,
                None,
            )
            .expect("preference leaves the device recipe usable");
            assert_eq!(descriptor.required_limits, limits);
            assert!(descriptor.required_features.is_empty());
            assert!(
                selections[0]
                    .declined
                    .as_ref()
                    .expect("reported reason")
                    .contains(reason)
            );
        }
    }
}
