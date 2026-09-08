//! Dashboard keyboard navigation, routed over the actor bus.
//!
//! `j`/`k`/`g`/`G` on the dashboard tab are commands to the application,
//! not keystroke composition, so they travel the fabric: the keymap
//! produces a `DashboardSelect*` intent, the feature's route row (see
//! [`crate::common::slices::key_routes::KeyRoutes`]) maps it to a
//! [`DashboardNav`], and the bus delivers it to the dashboard actor —
//! its sole subscriber, which folds the navigation into the slice cell.
//!
//! Sole-subscriber note: kameo's bus is broadcast, so "routing to the
//! dashboard actor" relies on it being the only subscriber for this
//! type. That pairing is asserted by test; keep `DashboardNav` reserved
//! for the dashboard actor's consumption.

use serde::{Deserialize, Serialize};

use crate::BusMessage;

/// Move the dashboard's selection cursor.
///
/// Published by the intent router on behalf of the dashboard feature's
/// keybind rows; consumed only by
/// [`DashboardActor`](super::dashboard_actor::DashboardActor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DashboardNav {
    /// Move selection up one entry (`k`).
    Up,
    /// Move selection down one entry (`j`).
    Down,
    /// Jump to the first entry (`g`).
    First,
    /// Jump to the last entry (`G`).
    Last,
}

impl BusMessage for DashboardNav {}
