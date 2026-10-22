//! Logging initialisation.
//!
//! `tracing` writes to stderr. The audio callback never logs; worker and UI code
//! use spans and events instead.

use tracing_subscriber::EnvFilter;

use crate::CoreError;

/// Default filter when neither the argument nor `ALP_LOG` is set.
pub const DEFAULT_FILTER: &str = "info";

/// Install the global tracing subscriber.
///
/// The filter comes from `filter`, then `ALP_LOG`, then [`DEFAULT_FILTER`].
pub fn init(filter: Option<&str>) -> Result<(), CoreError> {
    let filter = filter
        .map(str::to_owned)
        .or_else(|| std::env::var("ALP_LOG").ok())
        .unwrap_or_else(|| DEFAULT_FILTER.to_owned());
    let env_filter = EnvFilter::try_new(&filter)
        .map_err(|error| CoreError::Logging(format!("invalid filter {filter:?}: {error}")))?;
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .with_target(true)
        .try_init()
        .map_err(|error| CoreError::Logging(error.to_string()))
}
