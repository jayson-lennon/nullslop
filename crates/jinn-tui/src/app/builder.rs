//! Builder for constructing a [`TuiApp`] with sensible defaults for tests.

use jinn_domain::AppCore;

use super::TuiApp;

/// Builder for constructing a [`TuiApp`] with sensible defaults for tests.
///
/// All fields default to fake/noop implementations. Override only what the test needs.
///
/// See the tests in this crate for usage patterns.
#[derive(Default)]
pub struct TuiAppBuilder {
    /// Optional services override (defaults to fake services).
    services: Option<jinn_domain::Services>,
    /// Optional app state override (defaults to default state).
    state: Option<jinn_domain::AppState>,
}

impl TuiAppBuilder {
    /// Override the default services.
    #[must_use]
    pub fn services(mut self, services: jinn_domain::Services) -> Self {
        self.services = Some(services);
        self
    }

    /// Override the default app state.
    #[must_use]
    pub fn state(mut self, state: jinn_domain::AppState) -> Self {
        self.state = Some(state);
        self
    }

    /// Build the `TuiApp` with the configured overrides.
    ///
    /// Delegates to [`crate::launch::launch_for_test`] so that the test path and
    /// the real launch path ([`crate::launch::launch`]) share a single keymap
    /// bootstrap site. This is what prevents test/prod divergence in keymap binding.
    pub async fn build(self) -> TuiApp {
        let services = match self.services {
            Some(s) => s,
            None => {
                let services = jinn_domain::Services::new_fake().await;
                // The test path must mint the same slice cell the real
                // wiring does, or the dashboard view's startup pairing
                // check (and every dashboard render) has nothing to read.
                // A fresh registry cannot fail; on a wiring regression the
                // pairing check in the launch path reports it instead.
                let dashboard_cell = services.slices.register(
                    jinn_domain::feat::dashboard::dashboard_slot(),
                    jinn_domain::feat::dashboard::DashboardState::new(),
                );
                match dashboard_cell {
                    Ok(cell) => {
                        cell.update(|d| d.mark_running("discord", Some("Discord bot".to_owned())));
                    }
                    Err(_) => {}
                }
                services
            }
        };
        let state = self.state.unwrap_or_default();

        let core = AppCore {
            state: jinn_domain::State::new(state),
            bridge: services.bridge.clone(),
        };

        crate::launch::launch_for_test(core, services)
    }
}
