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

#[doc(inline)]
pub use cemi::*;
#[doc(inline)]
pub use constants::*;
#[doc(inline)]
pub use frame::*;
#[doc(inline)]
pub use services::*;
#[doc(inline)]
pub use tunnel::*;

#[cfg(any(feature = "embassy-rp", feature = "embassy-rp-usb"))]
#[doc(inline)]
pub use async_tunnel::*;
