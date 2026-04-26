//! nullslop-extension: SDK for building nullslop extensions.
//!
//! Extension authors implement the [`Extension`] trait and use the
//! [`run!`] macro to generate their binary's `main()` function.

pub mod codec;
pub mod context;
pub mod extension;
pub mod macros;

pub use codec::{InboundMessage, OutboundMessage};
pub use context::Context;
pub use extension::Extension;
