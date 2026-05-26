//! Admin HTTP middleware.

pub mod cf_access;

pub use cf_access::{cf_access_middleware, CfAccessError, CfAccessValidator};

#[doc(hidden)]
pub use cf_access::{test_valid_jwt, test_validator};
