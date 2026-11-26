//! GPU weather products. The game supplies the field's type data and committed state.
mod field_overlay;
mod ground;
mod layer;
pub use field_overlay::FieldOverlay;
mod noise;
mod shade;
mod shapes;
mod sun_path;
mod weather;

pub use ground::{GroundConfig, GroundField, GroundProducts};
pub use layer::{SkyProducts, SkyProfile};
#[cfg(target_arch = "wasm32")]
pub use shade::CloudSubmission;
pub use shade::{Shade, ShadeConfig, WeatherInput};
pub use shapes::{ShapesConfig, ShapesReport};
pub use sun_path::{SunLevel, SunPath, SunProfile};
pub use weather::{RowStamp, WeatherMap, WeatherProfile};

/// A rejected GPU product configuration.
#[derive(Debug)]
pub enum Error {
    /// The presentation mailbox is unavailable.
    InputUnavailable,
    /// Invalid CPU weather data.
    Model(thrum_cloud_model::Error),
    /// An authored dimension or precision range is unsupported.
    Unsupported(&'static str),
    /// The prepared device has insufficient capacity for a cloud product.
    DeviceLimit(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputUnavailable => f.write_str("cloud presentation mailbox is unavailable"),
            Self::Model(error) => write!(f, "cloud model: {error}"),
            Self::Unsupported(reason) | Self::DeviceLimit(reason) => {
                write!(f, "cloud GPU product: {reason}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Model(error) => Some(error),
            Self::Unsupported(_) | Self::DeviceLimit(_) | Self::InputUnavailable => None,
        }
    }
}
