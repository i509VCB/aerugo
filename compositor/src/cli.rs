//! Command line argument parsing using clap.

use clap::{Parser, ValueEnum};

/// The Aerugo wayland compositor
#[deny(missing_docs)]
#[derive(Parser, Debug)]
#[clap(about = "A Wayland compositor written in Rust", author, version)]
pub struct AerugoArgs {
    /// Backend selection
    ///
    /// By default the backend will be selected depending on the environment (`auto`).
    ///
    /// There are two primary backends that may be chosen:
    ///
    /// `kms`: Used when the compositor is run under a session. This backend is used when aerugo is your primary display
    /// server.
    ///
    /// `windowed`: The compositor is run inside a window as an X11 or Wayland client. The windowed backend is useful
    /// for testing purposes.
    ///
    /// The `x11` and `wayland` options both act like `windowed`, but allow specifying whether aerugo is run as an X11
    /// or Wayland client.
    #[clap(value_enum, default_value_t, short, long)]
    pub backend: Backend,

    /// Renderer selection
    ///
    /// This allows overriding the renderer to use at runtime. This may be useful in case of driver bugs.
    ///
    /// Right now only the OpenGL ES renderer is supported. In the future a Vulkan renderer will be available.
    #[clap(value_enum, default_value_t, long)]
    pub renderer: Renderer,
    // TODO: WM process to start
    // TODO: How should the WM spawn privileged clients?
}

/// Enum containing all possible backend selections.
#[deny(missing_docs)]
#[derive(ValueEnum, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Automatically choose the backend depending on the environment.
    #[default]
    Auto,

    /// Launch the compositor using kernel mode setting.
    ///
    /// This should be used if you launch the compositor from a TTY.
    #[clap(alias("tty"))]
    Kms,

    /// Launch the compositor inside a window.
    ///
    /// This will select Wayland or X11 as appropriate.
    Windowed,

    /// Launch the compositor inside a window as a Wayland client.
    #[clap(alias("wl"))]
    Wayland,

    /// Launch the compositor inside a window as an X11 client.
    #[clap(alias("x"))]
    X11,
}

/// Enum containing all possible renderer backends
#[deny(missing_docs)]
#[derive(ValueEnum, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    /// Select the most optimal, supported renderer.
    #[default]
    Default,

    /// Use the OpenGL ES renderer.
    #[clap(alias("egl"))]
    #[clap(alias("gl"))]
    Gles,
    // #[clap(alias("vk"))]
    // Vulkan, // TODO
}
