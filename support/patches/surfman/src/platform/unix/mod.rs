// surfman/surfman/src/platform/unix/mod.rs
//
//! Backends specific to Unix-like systems, particularly Linux.

// Force the appliance build to use the X11-oriented default backend even when
// Surfman is compiled with `wayland_default`, because Servo is launched under
// X11/Xwayland in QEMU and the Wayland default path crashes during bootstrap.
#[cfg(x11_platform)]
pub mod default;

#[cfg(free_unix)]
pub mod generic;

#[cfg(wayland_platform)]
pub mod wayland;
#[cfg(x11_platform)]
pub mod x11;
