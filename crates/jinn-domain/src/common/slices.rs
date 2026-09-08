//! Re-export shim: the slice vocabulary moved to the `jinn-slices` crate.
//!
//! This shim keeps existing `crate::common::slices::…` imports compiling
//! during the extraction. New code should import `jinn_slices` directly;
//! the shim carries no items of its own and is deleted in a later cleanup.

pub mod key_routes;

pub use jinn_slices::cell;
pub use jinn_slices::slices;
pub use jinn_slices::view;

pub use jinn_slices::SliceView;
pub use jinn_slices::Slices;
pub use jinn_slices::SlotKey;
pub use jinn_slices::SlotTaken;
pub use jinn_slices::TypedCell;
pub use jinn_slices::ViewCx;
