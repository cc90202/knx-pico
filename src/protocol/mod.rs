//! KNXnet/IP protocol implementation.
//!
//! This module contains the core protocol structures and parsing logic
//! for KNXnet/IP frames, cEMI messages, and KNX telegrams.

pub mod cemi;
pub mod constants;
pub mod frame;
pub mod services;
pub mod tunnel;

// Async tunnel client (requires embassy-net)
#[cfg(feature = "embassy-rp")]
pub mod async_tunnel;

pub use cemi::*;
pub use constants::*;
pub use frame::*;
pub use services::*;
pub use tunnel::*;

#[cfg(feature = "embassy-rp")]
pub use async_tunnel::*;
