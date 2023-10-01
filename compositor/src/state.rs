use std::{
    fmt,
    time::{Duration, SystemTime},
};

use bitflags::bitflags;
use calloop::LoopHandle;
use smithay::{
    input::SeatState,
    output::{Output, PhysicalProperties},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        shell::xdg::XdgShellState,
    },
};
use wayland_server::{
    backend::{ClientId, DisconnectReason},
    Client, DisplayHandle,
};

use crate::{
    backend::Backend,
    scene::Scene,
    shell::Shell,
    wayland::{ext::foreign_toplevel::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, versions},
    Loop,
};

#[derive(Debug)]
pub struct Aerugo {
    pub display: DisplayHandle,
    pub shell: Shell,
    pub scene: Scene,
    // This is not what I want in the future, but is for testing.
    pub output: Output,
    pub backend: Box<dyn Backend>,
    pub wl_compositor: CompositorState,
    pub xdg_shell: XdgShellState,
    pub seat_state: SeatState<Self>,
    pub generation: u64,
}

impl Aerugo {
    pub fn new(_loop: &LoopHandle<'static, Loop>, display: DisplayHandle, backend: Box<dyn Backend>) -> Self {
        // Initialize common globals
        let seat_state = SeatState::new();
        let wl_compositor = CompositorState::new::<Self>(&display);
        let xdg_shell = XdgShellState::new::<Self>(&display);
        let _foreign_toplevel_list =
            display.create_global::<Self, ExtForeignToplevelListV1, _>(versions::EXT_FOREIGN_TOPLEVEL_LIST_V1, ());
        let output = Output::new(
            "Test output".into(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: smithay::output::Subpixel::Unknown,
                make: String::new(),
                model: String::new(),
            },
        );
        output.create_global::<Self>(&display);

        let mut scene = Scene::new();
        scene.create_output(output.clone());

        let shell = Shell::new();

        let generation = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .as_ref()
            .map(Duration::as_secs)
            // If the system time is messed up, pick some predefined generation timestamp.
            .unwrap_or(u64::MAX);

        Self {
            display,
            wl_compositor,
            xdg_shell,
            seat_state,
            shell,
            scene,
            output,
            backend,
            generation,
        }
    }
}

bitflags! {
    /// Bitflag to describe what globals are visible to clients.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct PrivilegedGlobals: u32 {
        /// Whether the `ext-foreign-toplevel-list-v1` global is available.
        const FOREIGN_TOPLEVEL_LIST = 0x01;

        /// Whether the `ext-foreign-toplevel-state-v1` global is available.
        ///
        /// This protocol is always enabled with the `ext-foreign-toplevel-list-v1` protocol.
        ///
        /// This is not enabled at the moment since the protocol is not yet done: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/196
        const FOREIGN_TOPLEVEL_STATE = 0x03;

        /// Whether the foreign toplevel management global is available.
        ///
        /// This protocol is always enabled with the `ext-foreign-toplevel-state-v1` protocol.
        const FOREIGN_TOPLEVEL_MANAGEMENT = 0x07;

        /// Whether the client is XWayland.
        ///
        /// This will enable the `xwayland-shell-v1` and `zwp_xwayland-keyboard-grab-v1` protocols.
        const XWAYLAND = 0x08;

        /// Whether the `ext-session-lock-v1` global is available.
        const SESSION_LOCK = 0x10;

        /// Whether the `zwlr-layer-shell-v1` protocol is available.
        ///
        /// This will also make the `ext-layer-shell-v1` protocol available when merged: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/28
        const LAYER_SHELL = 0x20;

        /// Whether the `aerugo-shell-v1` protocol is available.
        const AERUGO_SHELL = 0x40;
    }
}

#[derive(Debug)]
pub struct ClientData {
    // TODO: Make private
    pub(super) globals: PrivilegedGlobals,
    pub(super) compositor: CompositorClientState,
}

impl ClientData {
    pub fn get_data(client: &Client) -> Option<&Self> {
        client.get_data()
    }

    pub fn client_compositor_state(&self) -> &CompositorClientState {
        &self.compositor
    }

    pub fn is_visible(&self, global: PrivilegedGlobals) -> bool {
        self.globals.contains(global)
    }
}

impl wayland_server::backend::ClientData for ClientData {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}
