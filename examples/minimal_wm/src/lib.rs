use std::collections::HashMap;

use aerugo::wm::types::{
    KeyFilter, KeyModifiers, KeyStatus, Output, OutputId, Server, Snapshot, Toplevel, ToplevelConfigure, ToplevelId,
    ToplevelUpdates,
};
use exports::aerugo::wm::wm_types::{Guest, GuestWm, WmInfo};
use wit_bindgen::{rt::string::String, Resource};
use xkeysym::KeyCode;

pub struct Wm {
    /// All known toplevels.
    toplevels: HashMap<ToplevelId, Toplevel>,
}

impl Wm {
    fn new(_server: Server) -> Self {
        todo!()
    }

    fn new_toplevel(&mut self, toplevel: Toplevel) {
        let id = toplevel.id();
        let toplevel = self.toplevels.entry(id).or_insert(toplevel);

        let _configure = ToplevelConfigure::new(toplevel);
    }

    fn closed_toplevel(&mut self, toplevel: ToplevelId) {
        // The wm may keep the toplevel around for animations. For the example drop the toplevel handle.
        self.toplevels.remove(&toplevel);
    }

    fn update_toplevel(&mut self, _toplevel: ToplevelId, _updates: ToplevelUpdates) {
        todo!()
    }

    fn ack_toplevel(&mut self, _toplevel: ToplevelId, _serial: u32) {
        todo!()
    }

    fn committed_toplevel(&mut self, _toplevel: ToplevelId, _snapshot: Option<Snapshot>) {
        todo!()
    }

    fn key(&mut self, _time: u32, _key_code: KeyCode, _compose: Option<String>, _status: KeyStatus) -> KeyFilter {
        todo!()
    }

    fn key_modifiers(&mut self, __modifiers: KeyModifiers) {
        todo!()
    }

    fn new_output(&mut self, __output: Output) {
        todo!()
    }

    fn disconnect_output(&mut self, __output: OutputId) {
        todo!()
    }
}

wit_bindgen::generate!({
    path: "../../wm.wit",

    world: "aerugo-wm",

    exports: {
        "aerugo:wm/wm-types": WmImpl,
        "aerugo:wm/wm-types/wm": WmImpl,
    },
});

pub struct WmImpl(std::cell::RefCell<Wm>);

impl Guest for WmImpl {
    fn get_info() -> Result<WmInfo, String> {
        Ok(WmInfo {
            abi_major: 0,
            abi_minor: 1,
            name: "minimal wm".into(),
            version: "none".into(),
        })
    }

    fn create_wm(server: Server) -> Result<Resource<WmImpl>, String> {
        let wm = Wm::new(server);
        Ok(Resource::new(Self(std::cell::RefCell::new(wm))))
    }
}

impl GuestWm for WmImpl {
    fn new_toplevel(&self, toplevel: Toplevel) {
        self.0.borrow_mut().new_toplevel(toplevel);
    }

    fn closed_toplevel(&self, toplevel: ToplevelId) {
        self.0.borrow_mut().closed_toplevel(toplevel);
    }

    fn update_toplevel(&self, toplevel: ToplevelId, updates: ToplevelUpdates) {
        self.0.borrow_mut().update_toplevel(toplevel, updates);
    }

    fn ack_toplevel(&self, toplevel: ToplevelId, serial: u32) {
        self.0.borrow_mut().ack_toplevel(toplevel, serial);
    }

    fn committed_toplevel(&self, toplevel: ToplevelId, snapshot: Option<Snapshot>) {
        self.0.borrow_mut().committed_toplevel(toplevel, snapshot)
    }

    fn key(&self, time: u32, sym: u32, compose: Option<String>, status: KeyStatus) -> KeyFilter {
        self.0.borrow_mut().key(time, KeyCode::from(sym), compose, status)
    }

    fn key_modifiers(&self, modifiers: KeyModifiers) {
        self.0.borrow_mut().key_modifiers(modifiers)
    }

    fn new_output(&self, output: Output) {
        self.0.borrow_mut().new_output(output);
    }

    fn disconnect_output(&self, output: OutputId) {
        self.0.borrow_mut().disconnect_output(output);
    }
}
