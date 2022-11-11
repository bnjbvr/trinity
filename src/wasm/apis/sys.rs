use crate::wasm::GuestState;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "./wit/sys.wit",
    name: "sys"
});

pub(super) struct SysApi;

impl SysApi {
    pub fn link(
        id: usize,
        linker: &mut wasmtime::component::Linker<GuestState>,
    ) -> anyhow::Result<()> {
        sys::add_to_linker(linker, move |s| &mut s.imports[id].apis.sys)
    }
}

impl sys::Sys for SysApi {
    fn rand_u64(&mut self) -> anyhow::Result<u64> {
        Ok(rand::random())
    }
}
