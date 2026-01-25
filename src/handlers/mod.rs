//! HTTP request handlers.

mod example_error;
mod health;
pub mod v1;
mod version;

pub use example_error::{example_bad_request, example_internal_error, example_not_found};
pub use health::{livez, readyz};
pub use version::version;
