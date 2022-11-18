use redb::{ReadableTable as _, TableDefinition};

use crate::{wasm::GuestState, ShareableDatabase};

wit_bindgen_host_wasmtime_rust::generate!({
    import: "./wit/kv.wit",
    name: "kv_store"
});

pub(super) struct KeyValueStoreApi {
    db: ShareableDatabase,
    module_name: String,
}

impl KeyValueStoreApi {
    pub fn new(db: ShareableDatabase, module_name: &str) -> anyhow::Result<Self> {
        Ok(Self {
            db,
            module_name: module_name.to_owned(),
        })
    }

    pub fn link(
        id: usize,
        linker: &mut wasmtime::component::Linker<GuestState>,
    ) -> anyhow::Result<()> {
        kv::add_to_linker(linker, move |s| &mut s.imports[id].apis.kv_store)
    }
}

impl kv::Kv for KeyValueStoreApi {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        let table_def = TableDefinition::<[u8], [u8]>::new(&self.module_name);
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(table_def)?;
            table.insert(&key, &value)?;
        }
        txn.commit()?;
        Ok(())
    }

    fn get(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        let table_def = TableDefinition::<[u8], [u8]>::new(&self.module_name);
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(table_def) {
            Ok(table) => table,
            Err(err) => match err {
                redb::Error::DatabaseAlreadyOpen
                | redb::Error::InvalidSavepoint
                | redb::Error::Corrupted(_)
                | redb::Error::TableTypeMismatch(_)
                | redb::Error::DbSizeMismatch { .. }
                | redb::Error::TableAlreadyOpen(_, _)
                | redb::Error::OutOfSpace
                | redb::Error::Io(_)
                | redb::Error::LockPoisoned(_) => Err(err)?,
                redb::Error::TableDoesNotExist(_) => return Ok(None),
            },
        };
        Ok(table.get(&key)?.map(|val| val.to_vec()))
    }

    fn remove(&mut self, key: Vec<u8>) -> anyhow::Result<()> {
        let table_def = TableDefinition::<[u8], [u8]>::new(&self.module_name);
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(table_def)?;
            table.remove(&key)?;
        }
        txn.commit()?;
        Ok(())
    }
}
