use std::collections::HashMap;

use matrix_sdk::{
    ruma::{OwnedRoomAliasId, OwnedRoomId},
    Client,
};

pub(super) struct RoomResolver {
    client: Client,
    /// In-memory cache for the room alias to room id mapping.
    room_cache: HashMap<OwnedRoomAliasId, OwnedRoomId>,
}

impl RoomResolver {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            room_cache: Default::default(),
        }
    }

    pub fn resolve_room(&mut self, room: &str) -> anyhow::Result<Option<String>> {
        if !room.starts_with("#") && !room.starts_with("!") {
            // This is likely not meant to be a room.
            return Ok(None);
        }

        // Shortcut: if the room is already a room id, return it.
        if let Ok(room_id) = OwnedRoomId::try_from(room) {
            return Ok(Some(room_id.to_string()));
        };

        // Try to resolve the room alias; if it's not valid, we report an error to the caller here.
        let room_alias = OwnedRoomAliasId::try_from(room)?;

        // Try cache first...
        if let Some(cached) = self.room_cache.get(&room_alias) {
            return Ok(Some(cached.to_string()));
        }

        // ...but if it fails, sync query the server.
        let client = self.client.clone();
        let room_alias_copy = room_alias.clone();
        let result = futures::executor::block_on(async move {
            client.resolve_room_alias(&room_alias_copy).await
        })?;

        let room_id = result.room_id;
        self.room_cache.insert(room_alias, room_id.clone());
        Ok(Some(room_id.to_string()))
    }
}
