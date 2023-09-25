wit_bindgen::generate!({
    path: "wm.wit",
    world: "aerugo-wm",
    exports: {
        "aerugo:wm/wm": Wm,
        "aerugo:wm/wm/wm-res": WmRes
    },
});

use crate::exports::aerugo::wm::wm::Guest;
use crate::exports::aerugo::wm::wm::GuestWmRes;
use wit_bindgen::rt::Resource;

pub struct WmRes;

impl GuestWmRes for WmRes {
    fn name(&self) -> String {
        loop {}
    }
}

pub struct Wm;

impl Guest for Wm {
    fn create_wm() -> Result<Resource<WmRes>, String> {
        loop {}
        //
    }
}
