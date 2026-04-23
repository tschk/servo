// surfman/src/platform/unix/default.rs
//
//! The default backend for the Soliloquy appliance on Unix.
//!
//! We force the default backend to X11 instead of Surfman's dynamic
//! Wayland/X11/surfaceless selector because the appliance explicitly launches
//! Servo on X11 under Xwayland, and the multi-backend path is currently hanging
//! during connection bootstrap in QEMU.

/// X11 display server connections.
pub mod connection {
    pub type Connection = crate::platform::unix::x11::connection::Connection;
    pub type NativeConnection = crate::platform::unix::x11::connection::NativeConnection;
}

/// OpenGL rendering contexts.
pub mod context {
    pub type Context = crate::platform::unix::x11::context::Context;
    pub type ContextDescriptor = crate::platform::unix::x11::context::ContextDescriptor;
    pub type NativeContext = crate::platform::unix::x11::context::NativeContext;
}

/// Thread-local handles to devices.
pub mod device {
    pub type Adapter = crate::platform::unix::x11::device::Adapter;
    pub type Device = crate::platform::unix::x11::device::Device;
    pub type NativeDevice = crate::platform::unix::x11::device::NativeDevice;
}

/// Hardware buffers of pixels.
pub mod surface {
    pub type NativeWidget = crate::platform::unix::x11::surface::NativeWidget;
    pub type Surface = crate::platform::unix::x11::surface::Surface;
    pub type SurfaceTexture = crate::platform::unix::x11::surface::SurfaceTexture;
}
