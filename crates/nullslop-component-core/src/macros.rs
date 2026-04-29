//! Declarative macro for defining components that handle messages (commands and events).
//!
//! The [`define_handler!`] macro reduces boilerplate by generating:
//! - The handler struct definition (unit struct)
//! - `impl CommandHandler<C>` for each command entry
//! - `impl EventHandler<E>` for each event entry
//! - A `register(&self, bus: &mut Bus)` method
//!
//! Users provide method implementations in a separate `impl` block for full
//! IDE support (autocomplete, type checking, inline errors).

/// Define a component that handles messages (commands and events).
///
/// A component can be a command/event handler (defined via this macro),
/// a UI element (implementing `UiElement`), or both. This macro
/// generates the handler struct and typed dispatch wiring.
///
/// Generates:
/// - The handler struct definition (unit struct)
/// - `impl CommandHandler<C, AppState>` for each command entry (forwards `CommandAction` return value)
/// - `impl EventHandler<E, AppState>` for each event entry
/// - A `register(&self, bus: &mut Bus<AppState>)` method
///
/// # Syntax
///
/// ```ignore
/// define_handler! {
///     /// Optional doc comments.
///     pub struct MyHandler;
///
///     commands {
///         CmdTypeA: method_a,
///         CmdTypeB: method_b,
///     }
///
///     events {
///         EvtTypeX: method_x,
///     }
/// }
/// ```
///
/// # Handler methods
///
/// Command handler methods must have this signature:
/// `fn method(cmd: &C, state: &mut AppState, out: &mut Out) -> CommandAction`
///
/// Event handler methods must have this signature:
/// `fn method(evt: &E, state: &mut AppState, out: &mut Out)`
///
/// Command methods return `CommandAction` directly — the macro forwards the return value.
/// Event methods return `()`.
///
/// `AppState` is a bare identifier that resolves at the call site — it must be in scope
/// wherever this macro is invoked.
#[macro_export]
macro_rules! define_handler {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident;

        commands {
            $($cmd_type:ty: $cmd_method:ident),* $(,)?
        }

        events {
            $($evt_type:ty: $evt_method:ident),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        // Generate CommandHandler impls (forward return value)
        $(
            impl $crate::CommandHandler<$cmd_type, AppState> for $name {
                #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
                fn handle(
                    &self,
                    cmd: &$cmd_type,
                    state: &mut AppState,
                    out: &mut $crate::Out,
                ) -> ::nullslop_protocol::CommandAction {
                    Self::$cmd_method(cmd, state, out)
                }
            }
        )*

        // Generate EventHandler impls
        $(
            impl $crate::EventHandler<$evt_type, AppState> for $name {
                #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
                fn handle(
                    &self,
                    evt: &$evt_type,
                    state: &mut AppState,
                    out: &mut $crate::Out,
                ) {
                    Self::$evt_method(evt, state, out);
                }
            }
        )*

        // Generate register method
        impl $name {
            #[doc = concat!("Register all handlers with the bus.\n\n⚠️ This must be called during application startup. Add a `", stringify!($name), ".register(&mut bus);` call in the component registration section of `run.rs`.")]
            pub fn register(&self, bus: &mut $crate::Bus<AppState>) {
                $(
                    bus.register_command_handler::<$cmd_type, Self>(*self);
                )*
                $(
                    bus.register_event_handler::<$evt_type, Self>(*self);
                )*
            }
        }
    };
}
