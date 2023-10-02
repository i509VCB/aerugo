//! WM runtime implementation.
//!
//! This crate implements the wm runtime used by Aerugo.

use std::num::NonZeroU32;

use wasmtime::component::Resource;

use crate::{ConfigureUpdate, Id, IdError, IdType, WmRequest, WmState, WmToplevelConfigure};

use self::aerugo::wm::types::{
    DecorationMode, Features, Focus, Geometry, Host, HostOutput, HostServer, HostSnapshot, HostToplevel,
    HostToplevelConfigure, HostView, HostViewBuilder, Output, OutputId, ResizeEdge, Server, Size, Snapshot, Toplevel,
    ToplevelConfigure, ToplevelId, ToplevelState, View, ViewBuilder,
};

wasmtime::component::bindgen!(in "../../wm.wit");

impl Host for WmState {}

impl HostServer for WmState {
    fn set_keyboard_focus(&mut self, server: Resource<Server>, _focus: Focus) -> wasmtime::Result<()> {
        self.validate_id_server(&server)?;
        todo!()
    }

    fn set_pointer_focus(&mut self, server: Resource<Server>, _focus: Focus) -> wasmtime::Result<()> {
        self.validate_id_server(&server)?;
        todo!()
    }

    fn drop(&mut self, server: Resource<Server>) -> wasmtime::Result<()> {
        // TODO: What should happen if the server is dropped?
        self.validate_id_server(&server)?;
        todo!("server drop")
    }
}

impl HostViewBuilder for WmState {
    fn with_toplevel(
        &mut self,
        toplevel: Resource<Toplevel>,
        image: Resource<Snapshot>,
    ) -> wasmtime::Result<Resource<ViewBuilder>> {
        todo!()
    }

    fn build(&mut self, builder: Resource<ViewBuilder>) -> wasmtime::Result<Resource<View>> {
        todo!()
    }

    fn drop(&mut self, builder: Resource<ViewBuilder>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostView for WmState {
    fn drop(&mut self, node: Resource<View>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostOutput for WmState {
    fn id(&mut self, output: Resource<Output>) -> wasmtime::Result<OutputId> {
        todo!()
    }

    fn name(&mut self, output: Resource<Output>) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    fn geometry(&mut self, output: Resource<Output>) -> wasmtime::Result<Geometry> {
        todo!()
    }

    fn refresh_rate(&mut self, output: Resource<Output>) -> wasmtime::Result<u32> {
        todo!()
    }

    fn drop(&mut self, output: Resource<Output>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostToplevel for WmState {
    fn features(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Features> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.features)
    }

    fn id(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<ToplevelId> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.id.rep().get())
    }

    fn app_id(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<String>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.app_id.clone())
    }

    fn title(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<String>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.title.clone())
    }

    fn min_size(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<Size>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.min_size)
    }

    fn max_size(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<Size>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.max_size)
    }

    fn geometry(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<Geometry>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.geometry)
    }

    fn parent(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<ToplevelId>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.parent.map(Id::rep).map(Into::into))
    }

    fn state(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<ToplevelState> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.state)
    }

    fn decorations(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<DecorationMode> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.decorations)
    }

    fn resize_edge(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<ResizeEdge>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        Ok(toplevel.resize_edge)
    }

    fn request_close(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<()> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        let id = toplevel.id;

        let _ = self.sender.send(WmRequest::ToplevelRequestClose(id));
        Ok(())
    }

    fn drop(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<()> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        let id = toplevel.id;
        // TODO: Remove id from this side.

        let _ = self.sender.send(WmRequest::ToplevelDrop(id));
        Ok(())
    }
}

impl HostToplevelConfigure for WmState {
    fn new(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Resource<ToplevelConfigure>> {
        let toplevel = self.get_toplevel_res(&toplevel)?;
        let configure = WmToplevelConfigure {
            toplevel_id: toplevel.id,
            decorations: Default::default(),
            parent: Default::default(),
            state: Default::default(),
            size: Default::default(),
            bounds: Default::default(),
        };

        Ok(Resource::new_own(todo!("Allocate owned id for toplevel configure")))
    }

    fn submit(&mut self, configure: Resource<ToplevelConfigure>) -> wasmtime::Result<u32> {
        let _configure = self.get_toplevel_configure(&configure)?;
        todo!()
    }

    fn decorations(
        &mut self,
        configure: Resource<ToplevelConfigure>,
        decorations: DecorationMode,
    ) -> wasmtime::Result<()> {
        let configure = self.get_toplevel_configure(&configure)?;
        configure.decorations = Some(decorations);
        Ok(())
    }

    fn parent(
        &mut self,
        configure: Resource<ToplevelConfigure>,
        parent: Option<Resource<Toplevel>>,
    ) -> wasmtime::Result<()> {
        let configure = self.get_toplevel_configure(&configure)?;

        match parent {
            Some(parent) => {
                if parent.owned() {
                    todo!("propagate error")
                }

                let parent_id = NonZeroU32::new(parent.rep()).ok_or(IdError::ZeroId)?;
                configure.parent = ConfigureUpdate::Update(Some(Id(parent_id, IdType::Toplevel)));
                Ok(())
            }

            None => {
                configure.parent = ConfigureUpdate::Update(None);
                Ok(())
            }
        }
    }

    fn state(&mut self, configure: Resource<ToplevelConfigure>, states: ToplevelState) -> wasmtime::Result<()> {
        let configure = self.get_toplevel_configure(&configure)?;
        configure.state = Some(states);
        Ok(())
    }

    fn size(&mut self, configure: Resource<ToplevelConfigure>, size: Option<Size>) -> wasmtime::Result<()> {
        let configure = self.get_toplevel_configure(&configure)?;
        configure.size = ConfigureUpdate::Update(size);
        Ok(())
    }

    fn bounds(&mut self, configure: Resource<ToplevelConfigure>, bounds: Option<Size>) -> wasmtime::Result<()> {
        let configure = self.get_toplevel_configure(&configure)?;
        configure.bounds = ConfigureUpdate::Update(bounds);
        Ok(())
    }

    fn drop(&mut self, configure: Resource<ToplevelConfigure>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostSnapshot for WmState {
    fn size(&mut self, snapshot: Resource<Snapshot>) -> wasmtime::Result<Size> {
        todo!()
    }

    fn scale(&mut self, snapshot: Resource<Snapshot>) -> wasmtime::Result<f32> {
        todo!()
    }

    fn drop(&mut self, snapshot: Resource<Snapshot>) -> wasmtime::Result<()> {
        todo!()
    }
}
