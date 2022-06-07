use smithay::{
    delegate_compositor, delegate_data_device, delegate_layer_shell, delegate_seat, delegate_shm,
    delegate_xdg_decoration, delegate_xdg_shell,
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
        wayland_server::{
            protocol::{wl_buffer, wl_surface},
            DisplayHandle,
        },
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorHandler, CompositorState},
        data_device::{ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler},
        output::OutputManagerState,
        seat::{SeatHandler, SeatState},
        shell::{
            wlr_layer::{LayerShellRequest, WlrLayerShellHandler, WlrLayerShellState},
            xdg::{
                self,
                decoration::{XdgDecorationHandler, XdgDecorationManager},
                XdgRequest, XdgShellHandler, XdgShellState,
            },
        },
        shm::ShmState,
    },
};

/// The compositor state
#[derive(Debug)]
pub struct Aerugo {
    pub protocols: Protocols,
    pub running: bool,
}

impl Aerugo {
    pub fn new(dh: &DisplayHandle) -> Aerugo {
        Aerugo {
            protocols: Protocols::new(dh),
            running: true,
        }
    }
}

/// Delegate types for protocol implementations.
#[derive(Debug)]
pub struct Protocols {
    pub compositor: CompositorState,
    pub seat: SeatState<Aerugo>,
    pub output_manager: OutputManagerState,
    pub data_device: DataDeviceState,
    pub shm: ShmState,
    pub xdg_shell: XdgShellState,
    pub xdg_decoration: XdgDecorationManager,
    pub layer_shell: WlrLayerShellState,
    // TODO:
    // - xdg-activation
    // - tablet-manager
}

impl Protocols {
    pub fn new(dh: &DisplayHandle) -> Protocols {
        Protocols {
            compositor: CompositorState::new::<Aerugo, _>(dh, None),
            seat: SeatState::new(),
            // TODO: Enable xdg-output?
            //
            // I think XWayland likes to have xdg-output?
            output_manager: OutputManagerState::new(),
            data_device: DataDeviceState::new::<Aerugo, _>(dh, None),
            // TODO: Allow more formats.
            //
            // For now this is fine, as all gpus must support Argb8888 and Xrgb8888.
            //
            // Probably want to consider the following formats in lowest-common denominator list:
            // - Rgba8888/Rgbx8888 (Permutations of Argb8888/Xrgb8888)
            // - Bgra8888/Bgrx8888
            // - Abgr8888/Xbgr8888
            // - Argb2101010/Xrgb2101010 (Argb2101010/Xrgb2101010 and permutations)
            // - Abgr2101010/Xbgr2101010
            // - Bgra1010102/Bgrx1010102
            // - Rgba1010102/Rgbx1010102
            // - Some common YUV formats
            // - Higher fidelity formats? (Argb16161616(F)/Xrgb16161616(F) and permutations?)
            // - Lower fidelity formats? (Argb4444/Xrgb4444, Rgb888, Argb1555/Xrgb1555, etc?)
            // - Weird formats? (R8/10/12/16, Axbxgxrx106106106106?)
            //
            // Discussion on #wayland:
            // 1. Simon disagrees that a wl_shm feedback is needed.
            // 2. If an added gpu does not support any lowest common denominator formats then one of the
            //    following is needed:
            // a. Convert format in SW before import
            // - Import as a raw buffer and convert during copy from buffer to texture.
            // b. Format is importable; sending to a different gpu
            // - If dma import fails: Convert on originating gpu via shader, then export memory
            //   or export memory and import as buffer, converting during copy from buffer to texture.
            //
            // Allocate with capacity of 2 because Argb8888/Xrgb8888 are always added.
            shm: ShmState::new::<Aerugo, _>(dh, Vec::with_capacity(2), None),
            // TODO: xdg-shell and xdg-decoration, remove GlobalId in tuple, make it a member of the types.
            xdg_shell: XdgShellState::new::<Aerugo, _>(dh, None).0,
            xdg_decoration: XdgDecorationManager::new::<Aerugo, _>(dh, None).0,
            layer_shell: WlrLayerShellState::new::<Aerugo, _>(dh, None),
        }
    }
}

// Handler implementations

impl BufferHandler for Aerugo {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {
        todo!()
    }
}

impl AsRef<ShmState> for Aerugo {
    fn as_ref(&self) -> &ShmState {
        &self.protocols.shm
    }
}

delegate_shm!(Aerugo);

impl CompositorHandler for Aerugo {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.protocols.compositor
    }

    fn commit(&mut self, _dh: &DisplayHandle, _surface: &wl_surface::WlSurface) {
        todo!()
    }
}

delegate_compositor!(Aerugo);

impl SeatHandler for Aerugo {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.protocols.seat
    }
}

delegate_seat!(Aerugo);

impl DataDeviceHandler for Aerugo {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.protocols.data_device
    }
}

impl ClientDndGrabHandler for Aerugo {}
impl ServerDndGrabHandler for Aerugo {}

delegate_data_device!(Aerugo);

impl XdgShellHandler for Aerugo {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.protocols.xdg_shell
    }

    fn request(&mut self, _dh: &DisplayHandle, _request: XdgRequest) {
        todo!()
    }
}

delegate_xdg_shell!(Aerugo);

impl XdgDecorationHandler for Aerugo {
    fn new_decoration(&mut self, _dh: &DisplayHandle, _toplevel: xdg::ToplevelSurface) {
        todo!()
    }

    fn request_mode(
        &mut self,
        _dh: &DisplayHandle,
        _toplevel: xdg::ToplevelSurface,
        _mode: zxdg_toplevel_decoration_v1::Mode,
    ) {
        todo!()
    }

    fn unset_mode(&mut self, _dh: &DisplayHandle, _toplevel: xdg::ToplevelSurface) {
        todo!()
    }
}

delegate_xdg_decoration!(Aerugo);

impl WlrLayerShellHandler for Aerugo {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.protocols.layer_shell
    }

    fn request(&mut self, _dh: &DisplayHandle, _request: LayerShellRequest) {
        todo!()
    }
}

delegate_layer_shell!(Aerugo);
