//! Common modules for examples
//!
//! This file includes the configuration module needed by examples.
//! Note: `knx_client` and `knx_discovery` are now available directly from `knx_pico` crate.

#![allow(dead_code, reason = "Common module shared across examples")]

// Configuration is user-specific, so we use the example template
// Users should copy configuration.rs.example to configuration.rs and update it
#[path = "../src/configuration.rs"]
pub mod configuration;

#[path = "../src/utility.rs"]
pub mod utility;

