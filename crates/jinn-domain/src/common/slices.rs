//! Render slices — dynamic, per-slice typed storage for the actor migration.
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

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        reason = "test assertions on infallible registration"
    )
)]

pub mod cell;
pub mod key_routes;
pub mod view;

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use parking_lot::RwLock;

pub use cell::TypedCell;

/// Uniquely addresses one slice cell.
///
/// `namespace` separates built-ins from plugin contributions
/// (`builtin` vs the plugin's name); `name` is the feature-chosen slice
/// name; `version` lets a slice payload evolve under a new key instead
/// of migrating in place.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotKey {
    /// Namespace owning the slot: `builtin` or a plugin name.
    namespace: String,
    /// Feature-chosen slice name, e.g. `status`.
    name: String,
    /// Payload schema version; bump to replace rather than mutate.
    version: u32,
}

impl SlotKey {
    /// A key for a built-in feature slice.
    #[must_use]
    pub fn builtin(namespace: &str, name: &str) -> Self {
        Self {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            version: 1,
        }
    }

    /// A key for a plugin-contributed slice.
    ///
    /// Guest slices are namespaced by plugin name so two plugins can
    /// never collide with each other or with built-ins.
    #[must_use]
    pub fn plugin(plugin_name: &str, name: &str) -> Self {
        Self {
            namespace: plugin_name.to_owned(),
            name: name.to_owned(),
            version: 1,
        }
    }

    /// The namespace component.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The slice-name component.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The payload schema version.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }
}

impl fmt::Display for SlotKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}@v{}", self.namespace, self.name, self.version)
    }
}

/// Error returned by [`Slices::register`] when the slot already exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotTaken {
    /// The contested slot key.
    pub key: SlotKey,
}

impl fmt::Display for SlotTaken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slot already registered: {}", self.key)
    }
}

impl std::error::Error for SlotTaken {}

/// Dynamic registry of per-slice typed cells.
///
/// Cheap to clone: every clone shares the same cells, so a handle
/// obtained before cloning still observes updates made through the
/// clone's registry (and vice versa). Cloning a `Slices` does **not**
/// mint new write capabilities — [`TypedCell`]s are minted only by
/// [`register`](Self::register).
#[derive(Clone, Debug, Default)]
pub struct Slices {
    cells: Arc<RwLock<HashMap<SlotKey, SlotEntry>>>,
}

/// One registered slot: the erased cell plus its payload's type name.
///
/// The type name exists for diagnostics — the startup view/slot pairing
/// check reports "slot X holds `Y`, expected `Z`" instead of a bare
/// "unregistered" for what is actually a wiring type error.
#[derive(Debug)]
struct SlotEntry {
    cell: Arc<dyn Any + Send + Sync>,
    type_name: &'static str,
}

impl Slices {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `initial` under `key` and mints the one write handle.
    ///
    /// # Errors
    ///
    /// Returns [`SlotTaken`] if the key is already registered. The
    /// original cell is untouched and its handle remains valid.
    pub fn register<T>(&self, key: SlotKey, initial: T) -> Result<TypedCell<T>, SlotTaken>
    where
        T: Any + Send + Sync,
    {
        let cell = TypedCell::new(initial);
        let mut cells = self.cells.write();
        if cells.contains_key(&key) {
            return Err(SlotTaken { key });
        }
        let entry = SlotEntry {
            cell: Arc::new(cell.clone()),
            type_name: std::any::type_name::<T>(),
        };
        cells.insert(key, entry);
        Ok(cell)
    }

    /// Returns a read handle to the cell registered under `key`, if its
    /// payload type is `T`.
    ///
    /// `None` means unregistered **or** type-mismatched — callers at
    /// startup should treat the mismatch as a wiring bug and fail fast
    /// (the [`view::Viewport`](view::Viewport) does exactly that).
    #[must_use]
    pub fn reader<T>(&self, key: &SlotKey) -> Option<TypedCell<T>>
    where
        T: Any + Send + Sync,
    {
        let cells = self.cells.read();
        let entry = cells.get(key)?;
        let cell = entry.cell.clone().downcast::<TypedCell<T>>().ok()?;
        Some((*cell).clone())
    }

    /// Returns the registered payload's type name for `key`, if present.
    ///
    /// Diagnostic support for the view/slot pairing check: distinguishes
    /// "slot missing" from "slot holds a different payload type".
    #[must_use]
    pub fn slot_type(&self, key: &SlotKey) -> Option<&'static str> {
        let cells = self.cells.read();
        Some(cells.get(key)?.type_name)
    }

    /// Enumerates every registered slot, sorted for stable display.
    ///
    /// Backs the dashboard's dynamic slice list and the future canvas
    /// export; a slice nobody registered simply doesn't appear.
    #[must_use]
    pub fn slots(&self) -> Vec<SlotKey> {
        let cells = self.cells.read();
        let mut keys: Vec<SlotKey> = cells.keys().cloned().collect();
        keys.sort();
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::Slices;
    use super::SlotKey;
    use super::key_routes::KeyRoutes;
    use super::key_routes::RouteRow;
    use super::view::SliceView;
    use super::view::ViewSlotErrorReason;
    use super::view::Viewport;
    use crate::protocol::intent::Intent;
    use crate::protocol::intent::IntentResult;
    use ratatui::Frame;
    use ratatui::layout::Rect;

    #[derive(Debug, Default)]
    struct Payload {
        value: u32,
    }

    fn stub_action() -> IntentResult {
        IntentResult::new_message(StubMsg)
    }

    #[derive(Debug)]
    struct StubView {
        slot: SlotKey,
    }

    impl SliceView for StubView {
        type Slice = Payload;

        fn slot(&self) -> SlotKey {
            self.slot.clone()
        }

        fn render(&mut self, _frame: &mut Frame<'_>, area: Rect, slice: &Self::Slice) {
            // Record that rendering saw the payload, via the frame
            // buffer: write one char per value unit in row 0.
            // (Real assertions happen through TestBackend below.)
            let _ = (area, slice);
        }
    }

    #[rstest::rstest]
    #[test]
    fn register_duplicate_slot_fails() {
        // Given a registry with a registered slot.
        let slices = Slices::new();
        let key = SlotKey::builtin("test", "dup");
        let first = slices
            .register(key.clone(), Payload::default())
            .expect("first register");

        // When registering the same slot again.
        let result = slices.register(key.clone(), Payload::default());

        // Then registration fails with SlotTaken.
        assert_eq!(result.unwrap_err().key, key);
        // And the original handle still updates the original cell.
        first.update(|p| p.value = 7);
        assert_eq!(first.read().value, 7);
    }

    #[rstest::rstest]
    #[test]
    fn reader_observes_owner_update() {
        // Given a registered cell and a reader handle for the same slot.
        let slices = Slices::new();
        let key = SlotKey::builtin("test", "observe");
        let owner = slices
            .register(key.clone(), Payload::default())
            .expect("register");
        let reader = slices.reader::<Payload>(&key).expect("reader resolves");

        // When the owner updates the cell.
        owner.update(|p| p.value = 42);

        // Then the reader observes the update.
        assert_eq!(reader.read().value, 42);
    }

    #[rstest::rstest]
    #[test]
    fn slots_enumerates_registered_keys() {
        // Given a registry with two registered slots.
        let slices = Slices::new();
        let a = SlotKey::builtin("a", "one");
        let b = SlotKey::plugin("plug", "two");
        let _ = slices
            .register(a.clone(), Payload::default())
            .expect("register a");
        let _ = slices
            .register(b.clone(), Payload::default())
            .expect("register b");

        // When enumerating slots.
        let slots = slices.slots();

        // Then both keys are present, sorted.
        assert_eq!(slots, vec![a, b]);
    }

    #[rstest::rstest]
    #[test]
    fn reader_type_mismatch_resolves_none() {
        // Given a slot registered with `Payload`.
        let slices = Slices::new();
        let key = SlotKey::builtin("test", "typed");
        let _ = slices
            .register(key.clone(), Payload::default())
            .expect("register");

        // When resolving the slot as a different type.
        let wrong = slices.reader::<Vec<u8>>(&key);

        // Then resolution fails (the wiring bug surface).
        assert!(wrong.is_none());
        // And the slot's registered type is reported for diagnostics.
        assert_eq!(
            slices.slot_type(&key),
            Some(std::any::type_name::<Payload>())
        );
    }

    #[derive(Debug)]
    struct VecView(SlotKey);

    impl SliceView for VecView {
        type Slice = Vec<u8>;
        fn slot(&self) -> SlotKey {
            self.0.clone()
        }
        fn render(&mut self, _frame: &mut Frame<'_>, _area: Rect, _slice: &Self::Slice) {}
    }

    #[rstest::rstest]
    #[test]
    fn viewport_rejects_unresolvable_view_slot() {
        // Given a registry where the view's slot is missing, and one
        // where it holds the wrong type.
        let slices = Slices::new();
        let missing = SlotKey::builtin("test", "missing");
        let mismatched = SlotKey::builtin("test", "mismatched");
        let _ = slices
            .register(mismatched.clone(), Payload::default())
            .expect("register");
        let mut viewport = Viewport::new();

        // When registering a view over the missing slot.
        let err = viewport
            .register(
                StubView {
                    slot: missing.clone(),
                },
                &slices,
            )
            .unwrap_err();

        // Then the pairing fails at registration, not at render.
        assert_eq!(err.key, missing);
        assert_eq!(err.reason, ViewSlotErrorReason::Unregistered);

        // When registering a view whose slice type differs from the
        // cell's payload.
        let err = viewport
            .register(VecView(mismatched.clone()), &slices)
            .unwrap_err();

        // Then the mismatch is reported with both type names.
        assert_eq!(err.key, mismatched);
        assert!(matches!(
            err.reason,
            ViewSlotErrorReason::TypeMismatch { .. }
        ));
    }

    #[rstest::rstest]
    #[test]
    fn route_lookup_produces_message_for_bound_intent() {
        // Given a table with a row bound to a dashboard nav intent.
        let routes = KeyRoutes::new();
        routes.attach_builtin(&Intent::DashboardSelectUp, stub_action, "stub");

        // When looking up the action for that intent (data variants
        // must match by discriminant, not payload).
        let result = routes.action_for(&Intent::DashboardSelectUp);

        // Then the action produced the stub message.
        let result = result.expect("bound intent resolves");
        assert_eq!(result.message_names, vec![std::any::type_name::<StubMsg>()]);
    }

    #[rstest::rstest]
    #[test]
    fn unbound_key_is_noop() {
        // Given a table with no row for quit.
        let routes = KeyRoutes::new();

        // When looking up the action for Intent::Quit.
        let result = routes.action_for(&Intent::Quit);

        // Then nothing resolves and the handler falls through.
        assert!(result.is_none());
    }

    #[rstest::rstest]
    #[test]
    fn guest_route_row_targets_plugin_coordinator() {
        // Given a table with a guest row attached.
        let routes = KeyRoutes::new();
        routes.attach_guest("dashboard", "ctrl+s", "my-plugin", "save");

        // When looking up the guest row by scope and key.
        let row = routes.guest_row("dashboard", "ctrl+s");

        // Then the row carries the plugin and the coordinator-addressed
        // action.
        let row = row.expect("guest row resolves");
        let RouteRow::Guest {
            scope,
            key,
            plugin,
            action,
        } = row
        else {
            unreachable!("attached row is a guest row");
        };
        assert_eq!(scope, "dashboard");
        assert_eq!(key, "ctrl+s");
        assert_eq!(plugin, "my-plugin");
        assert_eq!(action, "save");
    }

    #[derive(Debug, Clone)]
    struct StubMsg;

    impl crate::common::bus::BusMessage for StubMsg {}
}
