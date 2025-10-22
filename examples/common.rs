//! Common modules for examples
//!
//! This file includes the common modules needed by examples.
//! It's a workaround since examples cannot directly use `mod` from src/.

#![allow(dead_code)]

#[path = "../src/configuration.rs"]
pub mod configuration;

#[path = "../src/utility.rs"]
pub mod utility;

#[path = "../src/knx_client.rs"]
pub mod knx_client;

#[path = "../src/knx_discovery.rs"]
pub mod knx_discovery;

