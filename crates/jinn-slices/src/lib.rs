//! Slice vocabulary — typed cells, views, and the slots that name them.
//!
//! Today jinn's render state is one `AppState` guarded by a single
//! `RwLock`, with ~25 actors writing through TCaps tokens that gate
//! *where* in the struct an actor may write. [`Slices`] replaces that
//! convention with structure: each slice of render state (dashboard
//! status, quake bar, terminal screen, …) lives in its own typed cell,
//! `register` mints **exactly one** write handle for it, and everyone
//! else holds read handles. "Who can write this slice" becomes
//! grep-provable — find the handle, find the writer.
//!
//! Keys are dynamic strings ([`SlotKey`]), not an enum of known features,
//! so plugin-contributed slices are first-class residents: a WASM guest's
//! host-side coordinator can register a cell under the guest's namespace
//! exactly like a built-in feature does.
//!
//! Read access is not scarce; write access is.
//!
//! This crate is the future extraction seam for slice *features*: it
//! depends only on `jinn-theme` and `ratatui` (for the view layer) —
//! never on `jinn-domain`.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::panic,
        reason = "test assertions on infallible registration"
    )
)]

pub mod cell;
pub mod slice_scope;
pub mod slices;
pub mod view;

pub use cell::TypedCell;
pub use slice_scope::DynamicIntent;
pub use slice_scope::SliceScopeId;
pub use slices::Slices;
pub use slices::SlotKey;
pub use slices::SlotTaken;
pub use view::SliceView;
pub use view::ViewCx;
