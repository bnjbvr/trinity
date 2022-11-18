use std::collections::HashMap;

use matrix_sdk::{
    ruma::{OwnedRoomAliasId, OwnedRoomId},
    Client,
};

use crate::wasm::GuestState;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "./wit/sys.wit",
    name: "sys"
});

pub(super) struct SysApi {
    client: Client,
    /// In-memory cache for the room alias to room id mapping.
    room_cache: HashMap<OwnedRoomAliasId, OwnedRoomId>,
}

impl SysApi {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            room_cache: Default::default(),
        }
    }

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

    fn resolve_room(&mut self, room: String) -> anyhow::Result<Result<String, String>> {
        // Shortcut: if the room is already a room id, return it.
        if let Ok(room_id) = OwnedRoomId::try_from(room.as_str()) {
            return Ok(Ok(room_id.to_string()));
        };

        // Try to resolve the room alias.
        let room_alias = match OwnedRoomAliasId::try_from(room.as_str()) {
            Ok(r) => r,
            Err(err) => return Ok(Err(err.to_string())),
        };

        // Try cache first...
        if let Some(cached) = self.room_cache.get(&room_alias) {
            return Ok(Ok(cached.to_string()));
        }

        // ...but if it fails, sync query the server.
        let client = self.client.clone();
        let room_alias_copy = room_alias.clone();
        let response = futures::executor::block_on(async move {
            client.resolve_room_alias(&room_alias_copy).await
        });

        match response {
            Ok(result) => {
                let room_id = result.room_id;
                self.room_cache.insert(room_alias, room_id.clone());
                Ok(Ok(room_id.to_string()))
            }

            Err(err) => Ok(Err(err.to_string())),
        }
    }
}
