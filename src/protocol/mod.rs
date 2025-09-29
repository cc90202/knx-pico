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
#[cfg(any(feature = "embassy-rp", feature = "embassy-rp-usb"))]
pub mod async_tunnel;

pub use cemi::*;
pub use constants::*;
pub use frame::*;
pub use services::*;
pub use tunnel::*;

#[cfg(any(feature = "embassy-rp", feature = "embassy-rp-usb"))]
pub use async_tunnel::*;
