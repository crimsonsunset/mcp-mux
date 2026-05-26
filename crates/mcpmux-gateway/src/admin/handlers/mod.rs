//! Admin REST handlers (Phase 2+).

pub mod health;
pub mod error;
pub mod events;
pub mod oauth;
pub mod read;
pub mod write;

pub use health::health;
