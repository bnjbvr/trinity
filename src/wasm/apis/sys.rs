use crate::wasm::apis::sys::trinity::api::sys;
use crate::wasm::GuestState;

wasmtime::component::bindgen!({
    path: "./wit/sys.wit",
    world: "sys-world"
});

pub(super) struct SysApi;

impl SysApi {
    pub fn link(
        linker: &mut wasmtime::component::Linker<GuestState>,
    ) -> wasmtime::Result<()> {
        sys::add_to_linker(linker, |s| &mut s.apis.sys)
    }
}

impl sys::Host for SysApi {
    fn rand_u64(&mut self) -> u64 {
        rand::random()
    }
}
