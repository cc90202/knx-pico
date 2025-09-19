//! KNX addressing system.
//!
//! KNX uses two types of addresses:
//! - Individual addresses for physical devices (Area.Line.Device)
//! - Group addresses for logical grouping (Main/Middle/Sub or Main/Sub)

pub mod group;
pub mod individual;

pub use group::GroupAddress;
pub use individual::IndividualAddress;
