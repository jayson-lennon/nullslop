//! The `run!` macro for generating an extension's `main()` function.
//!
//! This macro generates a synchronous `main()` that:
//! 1. Creates a [`Context`](crate::Context)
//! 2. Calls `<ExtensionType>::activate(&mut ctx)` to construct the extension
//! 3. Flushes registrations/subscriptions to stdout
//! 4. Enters a blocking loop reading stdin lines:
//!    - `Command` → `extension.on_command()`
//!    - `Event` → `extension.on_event()`
//!    - `Shutdown` → `extension.deactivate()` then `break`
///    - `Initialize` → ignored (already initialized)
/// 5. On EOF → break (host closed stdin)
///
/// Generates a synchronous `main()` for an extension.
///
/// # Example
///
/// ```ignore
/// use nullslop_extension::{Extension, Context, run};
///
/// struct MyExtension;
///
/// impl Extension for MyExtension {
///     fn activate(ctx: &mut Context) -> Self {
///         ctx.register_command("hello");
///         Self
///     }
///     // ...
/// }
///
/// run!(MyExtension);
/// ```
#[macro_export]
macro_rules! run {
    ($extension_type:ty) => {
        fn main() {
            let mut ctx = $crate::Context::new(
            std::sync::Arc::new($crate::context::StdoutCommandSink),
            $crate::context::ContextKind::Process,
        );
            let mut extension = <$extension_type as $crate::Extension>::activate(&mut ctx);

            // Flush registrations.
            let (commands, subscriptions) = ctx.take_registrations();
            if !commands.is_empty() || !subscriptions.is_empty() {
                $crate::codec::write_message(&$crate::OutboundMessage::Register {
                    commands,
                    subscriptions,
                })
                .expect("failed to write registration");
            }

            // Main loop.
            while let Some(msg) = $crate::codec::read_message().expect("failed to read message") {
                match msg {
                    $crate::InboundMessage::Command { command } => {
                        <$extension_type as $crate::Extension>::on_command(
                            &mut extension,
                            &command,
                            &ctx,
                        );
                    }
                    $crate::InboundMessage::Event { event } => {
                        <$extension_type as $crate::Extension>::on_event(
                            &mut extension,
                            &event,
                            &ctx,
                        );
                    }
                    $crate::InboundMessage::Shutdown => {
                        <$extension_type as $crate::Extension>::deactivate(&mut extension);
                        break;
                    }
                    $crate::InboundMessage::Initialize => {
                        // Already initialized — ignore.
                    }
                }
            }
        }
    };
}
