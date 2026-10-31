//! Local control socket: request handling and the Unix listener.
//!
//! Compiled on Unix only (`mod control` is `#[cfg(unix)]` in `main.rs`).

pub mod handlers;
pub mod server;

pub use handlers::ControlState;
pub use server::ControlServer;
