//! GPU weather products. The game supplies the field's type data and committed state.
mod layer;
mod shade;
mod sun_path;
mod weather;

pub use layer::SkyProfile;
pub use shade::{Shade, ShadeConfig, WeatherInput};
pub use sun_path::{SunLevel, SunPath, SunProfile};
pub use weather::{RowStamp, WeatherMap, WeatherProfile};

/// A rejected GPU product configuration.
#[derive(Debug)]
pub enum Error {
    /// The presentation mailbox is unavailable.
    InputUnavailable,
    /// Invalid CPU weather data.
    Model(thrum_cloud_model::Error),
    /// A dimension, precision range or device limit is unsupported.
    Unsupported(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputUnavailable => f.write_str("cloud presentation mailbox is unavailable"),
            Self::Model(error) => write!(f, "cloud model: {error}"),
            Self::Unsupported(reason) => write!(f, "cloud GPU product: {reason}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Model(error) => Some(error),
            Self::Unsupported(_) | Self::InputUnavailable => None,
        }
    }
}
