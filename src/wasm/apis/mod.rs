mod kv_store;
mod log;
mod sync_request;
mod sys;

use crate::ShareableDatabase;

use self::kv_store::KeyValueStoreApi;
use self::log::LogApi;
use self::sync_request::SyncRequestApi;
use self::sys::SysApi;

pub(crate) struct Apis {
    sys: SysApi,
    log: LogApi,
    sync_request: SyncRequestApi,
    kv_store: KeyValueStoreApi,
}

impl Apis {
    pub fn new(module_name: String, db: ShareableDatabase) -> anyhow::Result<Self> {
        Ok(Self {
            sys: SysApi {},
            log: LogApi::new(&module_name),
            sync_request: SyncRequestApi::default(),
            kv_store: KeyValueStoreApi::new(db, &module_name)?,
        })
    }

    // Updated to use Apis as the store data type
    pub fn link(linker: &mut wasmtime::component::Linker<Apis>) -> anyhow::Result<()> {
        sys::SysApi::link(linker)?;
        log::LogApi::link(linker)?;
        sync_request::SyncRequestApi::link(linker)?;
        kv_store::KeyValueStoreApi::link(linker)?;
        Ok(())
    }
}
