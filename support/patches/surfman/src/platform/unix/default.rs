// surfman/src/platform/unix/default.rs
//
//! The default backend for the Soliloquy appliance on Unix.
//!
//! Headless contexts use surfaceless Mesa through `Connection::new()`, while
//! headed contexts use X11 through `Connection::from_display_handle()`.

type DefaultDevice = crate::platform::unix::generic::device::Device;
type AlternateDevice = crate::platform::unix::x11::device::Device;

/// X11 display server connections.
pub mod connection {
    use super::{AlternateDevice, DefaultDevice};

    /// Default Unix connection: surfaceless first, X11 fallback.
    pub type Connection =
        crate::platform::generic::multi::connection::Connection<DefaultDevice, AlternateDevice>;
    /// Native Unix connection for the selected backend.
    pub type NativeConnection = crate::platform::generic::multi::connection::NativeConnection<
        DefaultDevice,
        AlternateDevice,
    >;
}

/// OpenGL rendering contexts.
pub mod context {
    use super::{AlternateDevice, DefaultDevice};

    /// Default Unix OpenGL context for the selected backend.
    pub type Context =
        crate::platform::generic::multi::context::Context<DefaultDevice, AlternateDevice>;
    /// Context descriptor for the selected backend.
    pub type ContextDescriptor =
        crate::platform::generic::multi::context::ContextDescriptor<DefaultDevice, AlternateDevice>;
    /// Native EGL context wrapper for the selected backend.
    pub type NativeContext =
        crate::platform::generic::multi::context::NativeContext<DefaultDevice, AlternateDevice>;
}

/// Thread-local handles to devices.
pub mod device {
    use super::{AlternateDevice, DefaultDevice};

    /// Display adapter for the selected backend.
    pub type Adapter =
        crate::platform::generic::multi::device::Adapter<DefaultDevice, AlternateDevice>;
    /// Thread-local rendering device for the selected backend.
    pub type Device =
        crate::platform::generic::multi::device::Device<DefaultDevice, AlternateDevice>;
    /// Native rendering device for the selected backend.
    pub type NativeDevice =
        crate::platform::generic::multi::device::NativeDevice<DefaultDevice, AlternateDevice>;
}

/// Hardware buffers of pixels.
pub mod surface {
    use super::{AlternateDevice, DefaultDevice};

    /// Native window handle wrapper for the selected backend.
    pub type NativeWidget =
        crate::platform::generic::multi::surface::NativeWidget<DefaultDevice, AlternateDevice>;
    /// Rendering surface for the selected backend.
    pub type Surface =
        crate::platform::generic::multi::surface::Surface<DefaultDevice, AlternateDevice>;
    /// Surface texture for sharing rendered pixels.
    pub type SurfaceTexture =
        crate::platform::generic::multi::surface::SurfaceTexture<DefaultDevice, AlternateDevice>;
}
