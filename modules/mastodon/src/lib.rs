use std::collections::HashMap;

use libcommand::{impl_command, CommandClient, TrinityCommand};
use wit_log as log;
use wit_sync_request;

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct RoomConfig {
    admins: Vec<String>,
    token: String,
    base_url: String,
}

impl RoomConfig {
    fn is_admin(&self, author: &str) -> bool {
        self.admins.iter().any(|admin| *admin == author)
    }
}

struct Component;

impl TrinityCommand for Component {
    fn init(_config: HashMap<String, String>) {
        let _ = log::set_boxed_logger(Box::new(crate::log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
    }

    fn on_help(topic: Option<&str>) -> String {
        if let Some(topic) = topic {
            match topic {
                "admin" => r#"available admin commands:
- set-config #BASE_URL #TOKEN
- remove-config
- allow #USER_ID
- disallow #USER_ID
- list-posters"#
                    .into(),
                "toot" | "!toot" => "Toot a message with !toot MESSAGE".into(),
                _ => "i don't know this command!".into(),
            }
        } else {
            "Post mastodon statuses from Matrix! Help topics: admin, toot".to_owned()
        }
    }

    fn on_msg(client: &mut CommandClient, content: &str) {
        let Some(content) = content.strip_prefix("!toot").map(|rest| rest.trim()) else {
            return;
        };

        let author_id = client.from();
        let content: &str = &content;
        let room = client.room();

        let Ok(Some(mut config)) = wit_kv::get::<_, RoomConfig>(room) else {
            return client.respond("couldn't read room configuration (error or missing)");
        };

        if !config.is_admin(author_id) {
            return client.respond("you're not allowed to post, sorry!");
        }

        if !config.base_url.ends_with("/") {
            config.base_url.push('/');
        }
        config.base_url.push_str("api/v1/statuses");

        #[derive(serde::Serialize)]
        struct Request {
            status: String,
        }

        let body = serde_json::to_string(&Request {
            status: content.to_owned(),
        })
        .unwrap();

        let Some(resp) = wit_sync_request::Request::post(&config.base_url)
            .header("Authorization", &format!("Bearer {}", config.token))
            .header("Content-Type", "application/json")
            .body(&body)
            .run()
            .ok()
        else {
            return client.respond("didn't receive a response from the server");
        };

        if resp.status != wit_sync_request::ResponseStatus::Success {
            log::info!(
                "request failed with non-success status code:\n\t{:?}",
                resp.body
            );
            return client.respond("error when sending toot, see logs!".to_owned());
        }

        client.react_with_ok();
    }

    fn on_admin(client: &mut CommandClient, cmd: &str) {
        let room = client.room();
        let sender = client.from();

        if let Some(rest) = cmd.strip_prefix("set-config") {
            // Format: set-config BASE_URL TOKEN
            let mut split = rest.trim().split_whitespace();

            let Some(base_url) = split.next() else {
                return client.respond("missing base url");
            };
            let Some(token) = split.next() else {
                return client.respond("missing token");
            };

            let config = RoomConfig {
                admins: vec![sender.to_owned()],
                token: token.to_owned(),
                base_url: base_url.to_owned(),
            };

            if let Err(err) = wit_kv::set(&room, &config) {
                return client.respond(format!("writing to kv store: {err:#}"));
            }

            return client.react_with_ok();
        }

        if cmd.starts_with("remove-config") {
            // Format: remove-config
            if let Err(err) = wit_kv::remove(&room) {
                return client.respond(format!("writing to kv store: {err:#}"));
            }

            return client.react_with_ok();
        }

        if let Some(rest) = cmd.strip_prefix("allow") {
            // Format: allow USER_ID
            let mut split = rest.trim().split_whitespace();

            let Some(user_id) = split.next() else {
                return client.respond("missing user id");
            };

            let Ok(Some(mut current)) = wit_kv::get::<_, RoomConfig>(&room) else {
                return client.respond("couldn't read room config for room");
            };

            current.admins.push(user_id.to_owned());

            if let Err(err) = wit_kv::set(&room, &current) {
                return client.respond(format!("when writing to kv store: {err:#}"));
            }

            return client.react_with_ok();
        }

        if let Some(rest) = cmd.strip_prefix("disallow") {
            // Format: disallow USER_ID
            let mut split = rest.trim().split_whitespace();

            let Some(user_id) = split.next() else {
                return client.respond("missing user id");
            };

            let Ok(Some(mut current)) = wit_kv::get::<_, RoomConfig>(&room) else {
                return client.respond("couldn't read room config for room");
            };

            if let Some(idx) = current.admins.iter().position(|val| val == user_id) {
                current.admins.remove(idx);
            } else {
                return client.respond("admin not found");
            }

            if let Err(err) = wit_kv::set(&room, &current) {
                return client.respond(format!("when writing to kv store: {err:#}"));
            }

            return client.react_with_ok();
        }

        if cmd.starts_with("list-posters") {
            // Format: list-posters ROOM
            let Ok(Some(current)) = wit_kv::get::<_, RoomConfig>(&room) else {
                return client.respond("couldn't read room config, or no config for this room");
            };
            return client.respond(current.admins.join(", "));
        }

        client.respond("unknown command!");
    }
}

impl_command!();
