mod log;
mod sync_request;
mod sys;

use self::log::LogApi;
use self::sync_request::SyncRequestApi;
use self::sys::SysApi;

use super::GuestState;

pub(crate) struct Apis {
    sys: SysApi,
    log: LogApi,
    sync_request: SyncRequestApi,
}

impl Apis {
    pub fn new(module_name: String) -> Self {
        Self {
            sys: SysApi,
            log: LogApi::new(module_name),
            sync_request: SyncRequestApi::default(),
        }
    }

    pub fn link(
        id: usize,
        linker: &mut wasmtime::component::Linker<GuestState>,
    ) -> anyhow::Result<()> {
        sys::SysApi::link(id, linker)?;
        log::LogApi::link(id, linker)?;
        sync_request::SyncRequestApi::link(id, linker)?;
        Ok(())
    }
}
