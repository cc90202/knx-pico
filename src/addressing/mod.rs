//! KNX addressing system.
//!
//! KNX uses two types of addresses:
//! - Individual addresses for physical devices (Area.Line.Device)
//! - Group addresses for logical grouping (Main/Middle/Sub or Main/Sub)

pub mod group;
pub mod individual;

#[doc(inline)]
pub use group::GroupAddress;
#[doc(inline)]
pub use individual::IndividualAddress;
