// surfman/surfman/src/platform/unix/mod.rs
//
//! Backends specific to Unix-like systems, particularly Linux.

// Headed GL uses Wayland `from_display_handle` (see unix/default.rs); surfaceless stays the
// `Connection::new()` default for headless/tooling.
#[cfg(x11_platform)]
pub mod default;

#[cfg(free_unix)]
pub mod generic;

#[cfg(wayland_platform)]
pub mod wayland;
#[cfg(x11_platform)]
pub mod x11;
