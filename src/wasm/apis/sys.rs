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
}

impl SysApi {
    pub fn new(client: Client) -> Self {
        Self { client }
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

        let client = self.client.clone();
        let response =
            futures::executor::block_on(
                async move { client.resolve_room_alias(&room_alias).await },
            );

        match response {
            Ok(result) => {
                let room_id = result.room_id.to_string();
                Ok(Ok(room_id))
            }
            Err(err) => Ok(Err(err.to_string())),
        }
    }
}
