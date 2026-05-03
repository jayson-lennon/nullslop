//! Macro for declaring a message-handling component.
//!
//! The [`define_handler!`] macro lets actor authors declare which commands
//! and events a component handles in a single block. It produces the handler
//! struct, the trait implementations, and a `register` method that wires
//! everything into the bus. Authors write the actual handler methods in a
//! separate `impl` block, preserving full IDE support.

/// Declare a component that handles specific commands and events.
///
/// Produces the handler struct, trait implementations for every listed
/// command and event type, and a `register` method that wires the component
/// into the bus.
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
/// `fn method(cmd: &C, ctx: &mut HandlerContext<'_, AppState, Services>) -> CommandAction`
///
/// Event handler methods must have this signature:
/// `fn method(evt: &E, ctx: &mut HandlerContext<'_, AppState, Services>)`
///
/// Command methods return `CommandAction` directly — the macro forwards the return value.
/// Event methods return `()`.
///
/// `AppState` and `Services` are bare identifiers that resolve at the call site — they must be
/// in scope wherever this macro is invoked.
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
            impl $crate::CommandHandler<$cmd_type, AppState, Services> for $name {
                #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
                fn handle(
                    &self,
                    cmd: &$cmd_type,
                    ctx: &mut $crate::HandlerContext<'_, AppState, Services>,
                ) -> ::nullslop_protocol::CommandAction {
                    Self::$cmd_method(cmd, ctx)
                }
            }
        )*

        // Generate EventHandler impls
        $(
            impl $crate::EventHandler<$evt_type, AppState, Services> for $name {
                #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
                fn handle(
                    &self,
                    evt: &$evt_type,
                    ctx: &mut $crate::HandlerContext<'_, AppState, Services>,
                ) {
                    Self::$evt_method(evt, ctx);
                }
            }
        )*

        // Generate register method
        impl $name {
            #[doc = concat!("Register all handlers with the bus.\n\n⚠️ This must be called during application startup. Add a `", stringify!($name), ".register(&mut bus);` call in the component registration section of `run.rs`.")]
            pub fn register(&self, bus: &mut $crate::Bus<AppState, Services>) {
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
