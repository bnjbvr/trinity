use crate::wasm::GuestState;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "./wit/log.wit",
    name: "logs"
});

pub(super) struct LogApi {
    module_name: String,
}

impl LogApi {
    pub fn new(module_name: String) -> Self {
        Self { module_name }
    }

    pub fn link(
        id: usize,
        linker: &mut wasmtime::component::Linker<GuestState>,
    ) -> anyhow::Result<()> {
        log::add_to_linker(linker, move |s| &mut s.imports[id].apis.log)
    }
}

impl log::Log for LogApi {
    fn trace(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::trace!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn debug(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::debug!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn info(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::info!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn warn(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::warn!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn error(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::error!("{} - {msg}", self.module_name);
        Ok(())
    }
}
