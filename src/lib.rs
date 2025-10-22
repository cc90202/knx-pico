#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![expect(dead_code, reason = "Library under development, not all items used yet")]
#![doc = include_str!("../README.md")]

//! # knx-rs
//!
//! KNXnet/IP protocol implementation for embedded systems.
//!
//! This crate provides a `no_std` implementation of the KNXnet/IP protocol,
//! designed for use with Embassy async runtime on embedded microcontrollers.
//!
//! ## Features
//!
//! - KNXnet/IP tunneling support
//! - Common Datapoint Types (DPT)
//! - Individual and Group addressing
//! - Zero-copy parsing
//! - Async/await with Embassy
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_rs::{KnxClient, GroupAddress};
//!
//! // Connect to KNX gateway and send a command
//! let addr = GroupAddress::new(1, 2, 3).unwrap();
//! client.write_bool(addr, true).await?;
//! ```

pub mod addressing;
pub mod dpt;
pub mod error;
pub mod net;
pub mod protocol;

// Macro modules (must be declared before use)
#[macro_use]
pub mod macros;
#[macro_use]
pub mod logging;

// Re-export commonly used types
#[doc(inline)]
pub use addressing::{GroupAddress, IndividualAddress};
#[doc(inline)]
pub use dpt::{Dpt1, Dpt5, Dpt9, DptDecode, DptEncode};
#[doc(inline)]
pub use error::{KnxError, Result};
#[doc(inline)]
pub use net::Ipv4Addr;
