//! WM runtime implementation.
//!
//! This crate implements the wm runtime used by Aerugo.

use wasmtime::component::Resource;

use crate::{WmRequest, WmState};

use self::aerugo::wm::types::{
    DecorationMode, Features, Focus, Geometry, Host, HostImage, HostNode, HostNodeBuilder, HostOutput, HostServer,
    HostToplevel, HostToplevelConfigure, Image, Node, NodeBuilder, Output, OutputId, ResizeEdge, Server, Size,
    Toplevel, ToplevelConfigure, ToplevelId, ToplevelState,
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

impl HostNodeBuilder for WmState {
    fn with_toplevel(
        &mut self,
        toplevel: Resource<Toplevel>,
        image: Resource<Image>,
    ) -> wasmtime::Result<Resource<NodeBuilder>> {
        todo!()
    }

    fn build(&mut self, builder: Resource<NodeBuilder>) -> wasmtime::Result<Resource<Node>> {
        todo!()
    }

    fn drop(&mut self, builder: Resource<NodeBuilder>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostNode for WmState {
    fn drop(&mut self, node: Resource<Node>) -> wasmtime::Result<()> {
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
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.features)
    }

    fn id(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<ToplevelId> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.id.rep().get())
    }

    fn app_id(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<String>> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.app_id.clone())
    }

    fn title(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<String>> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.title.clone())
    }

    fn min_size(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<Size>> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.min_size)
    }

    fn max_size(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<Size>> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.max_size)
    }

    fn geometry(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<Geometry>> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.geometry)
    }

    fn parent(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<ToplevelId>> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.parent)
    }

    fn state(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<ToplevelState> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.state)
    }

    fn decorations(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<DecorationMode> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.decorations)
    }

    fn resize_edge(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Option<ResizeEdge>> {
        let toplevel = self.get_toplevel(&toplevel)?;
        Ok(toplevel.resize_edge)
    }

    fn request_close(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<()> {
        let toplevel = self.get_toplevel(&toplevel)?;
        let id = toplevel.id;

        let _ = self.sender.send(WmRequest::ToplevelRequestClose(id));
        Ok(())
    }

    fn drop(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<()> {
        let toplevel = self.get_toplevel(&toplevel)?;
        let id = toplevel.id;
        // TODO: Remove id from this side.

        let _ = self.sender.send(WmRequest::ToplevelDrop(id));
        Ok(())
    }
}

impl HostToplevelConfigure for WmState {
    fn new(&mut self, toplevel: Resource<Toplevel>) -> wasmtime::Result<Resource<ToplevelConfigure>> {
        todo!()
    }

    fn submit(&mut self, configure: Resource<ToplevelConfigure>) -> wasmtime::Result<u32> {
        todo!()
    }

    fn decorations(
        &mut self,
        configure: Resource<ToplevelConfigure>,
        decorations: DecorationMode,
    ) -> wasmtime::Result<()> {
        todo!()
    }

    fn parent(&mut self, configure: Resource<ToplevelConfigure>, parent: Resource<Toplevel>) -> wasmtime::Result<()> {
        todo!()
    }

    fn state(&mut self, configure: Resource<ToplevelConfigure>, states: ToplevelState) -> wasmtime::Result<()> {
        todo!()
    }

    fn size(&mut self, configure: Resource<ToplevelConfigure>, size: Option<Size>) -> wasmtime::Result<()> {
        todo!()
    }

    fn bounds(&mut self, configure: Resource<ToplevelConfigure>, bounds: Option<Size>) -> wasmtime::Result<()> {
        todo!()
    }

    fn drop(&mut self, configure: Resource<ToplevelConfigure>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostImage for WmState {
    fn size(&mut self, image: Resource<Image>) -> wasmtime::Result<Size> {
        todo!()
    }

    fn scale(&mut self, image: Resource<Image>) -> wasmtime::Result<f32> {
        todo!()
    }

    fn drop(&mut self, image: Resource<Image>) -> wasmtime::Result<()> {
        todo!()
    }
}