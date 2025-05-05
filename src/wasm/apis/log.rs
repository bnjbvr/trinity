use crate::wasm::apis::log::trinity::api::log;
use crate::wasm::ModuleState;

wasmtime::component::bindgen!({
    path: "./wit/log.wit",
    world: "log-world"
});

pub(super) struct LogApi {
    module_name: String,
}

impl LogApi {
    pub fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_owned(),
        }
    }

    pub fn link(linker: &mut wasmtime::component::Linker<ModuleState>) -> wasmtime::Result<()> {
        log::add_to_linker(linker, move |s| &mut s.apis.log)
    }
}

impl log::Host for LogApi {
    fn trace(&mut self, msg: String) {
        tracing::trace!("{} - {msg}", self.module_name);
    }
    fn debug(&mut self, msg: String) {
        tracing::debug!("{} - {msg}", self.module_name);
    }
    fn info(&mut self, msg: String) {
        tracing::info!("{} - {msg}", self.module_name);
    }
    fn warn(&mut self, msg: String) {
        tracing::warn!("{} - {msg}", self.module_name);
    }
    fn error(&mut self, msg: String) {
        tracing::error!("{} - {msg}", self.module_name);
    }
}
