use crate::wasm::apis::sys::trinity::api::sys;
use crate::wasm::GuestState;

wasmtime::component::bindgen!({
    path: "./wit/sys.wit",
    world: "sys-world"
});

pub(super) struct SysApi;

impl SysApi {
    pub fn link(
        id: usize,
        linker: &mut wasmtime::component::Linker<GuestState>,
    ) -> wasmtime::Result<()> {
        sys::add_to_linker(linker, move |s| &mut s.imports[id].apis.sys)
    }
}

impl sys::Host for SysApi {
    fn rand_u64(&mut self) -> u64 {
        rand::random()
    }
}
