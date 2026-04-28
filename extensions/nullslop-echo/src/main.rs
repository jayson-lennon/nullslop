//! Process-mode entry point for nullslop-echo extension.

use nullslop_echo::EchoExtension;
use nullslop_extension::run;

run!(EchoExtension);
