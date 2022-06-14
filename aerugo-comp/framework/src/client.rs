use std::{
    ffi::OsStr,
    io,
    os::unix::{net::UnixStream, prelude::AsRawFd},
    process::{Child, Command, Stdio},
};

/// A wrapper around [`Command`] for spawning Wayland and X11 clients.
///
/// The values for `WAYLAND_DISPLAY`, `WAYLAND_SOCKET` and `DISPLAY` will be cleared. Stdin, Stdout and Stderr
/// are also cleared.
#[derive(Debug)]
pub struct SpawnClient(Command);

impl SpawnClient {
    /// Builds a new [`SpawnClient`] to execute the specified program.
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        let mut command = Command::new(program);
        // Clear env that Wayland and X11 clients use to find the display server.
        command.env_remove("WAYLAND_DISPLAY");
        command.env_remove("WAYLAND_SOCKET");
        command.env_remove("DISPLAY");

        // Clear stdin, stdout and stderr
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        Self(command)
    }

    /// Sets the name of the Wayland display the client should connect to.
    ///
    /// This will set `WAYLAND_DISPLAY` on the new process.
    pub fn wayland_display<S: AsRef<OsStr>>(&mut self, socket: S) -> &mut Self {
        self.0.env("WAYLAND_DISPLAY", socket);
        self
    }

    /// Sets the socket of the Wayland display the client should connect to.
    ///
    /// This may be used to spawn a client with access to privileged protocols. The other end of the stream
    /// should be added as a client in the compositor.
    ///
    /// This will set `WAYLAND_SOCKET` on the new process.
    ///
    /// **Note:** Clients will prioritize `WAYLAND_SOCKET` over `WAYLAND_DISPLAY`.
    pub fn wayland_socket(&mut self, stream: UnixStream) -> &mut Self {
        self.0.env("WAYLAND_SOCKET", format!("{}", stream.as_raw_fd()));
        self
    }

    /// Sets the name of the XWayland server the client should connect to.
    pub fn x_display(&mut self, number: u32) -> &mut Self {
        self.0.env("DISPLAY", format!(":{}", number));
        self
    }

    /// Spawns the client, returning a handle to the child process.
    pub fn spawn(&mut self) -> io::Result<Child> {
        self.0.spawn()
    }

    /// Consumes the [`SpawnClient`], returning the underlying [`Command`].
    ///
    /// This may be used to add additional environment variables before spawning the client.
    pub fn command(self) -> Command {
        self.0
    }
}
