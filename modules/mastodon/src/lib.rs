use anyhow::Context as _;
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

impl Component {
    fn handle_msg(author_id: &str, content: &str, room: &str) -> anyhow::Result<String> {
        let mut config = wit_kv::get::<_, RoomConfig>(room)
            .context("couldn't read room configuration")?
            .context("missing room configuration")?;

        anyhow::ensure!(
            config.is_admin(author_id),
            "you're not allowed to post, sorry!"
        );

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
        })?;

        let resp = wit_sync_request::Request::post(&config.base_url)
            .header("Authorization", &format!("Bearer {}", config.token))
            .header("Content-Type", "application/json")
            .body(&body)
            .run()
            .ok()
            .context("no response")?;

        if resp.status != wit_sync_request::ResponseStatus::Success {
            log::info!(
                "request failed with non-success status code:\n\t{:?}",
                resp.body
            );
            anyhow::bail!("error when sending toot, see logs!");
        }

        Ok("great success!".to_owned())
    }

    fn handle_admin(cmd: &str, sender: &str, room: &str) -> anyhow::Result<String> {
        if let Some(rest) = cmd.strip_prefix("set-config") {
            // Format: set-config BASE_URL TOKEN
            let mut split = rest.trim().split_whitespace();

            let base_url = split.next().context("missing base url")?;
            let token = split.next().context("missing token")?;

            let config = RoomConfig {
                admins: vec![sender.to_owned()],
                token: token.to_owned(),
                base_url: base_url.to_owned(),
            };

            wit_kv::set(&room, &config).context("writing to kv store")?;

            return Ok("added!".to_owned());
        }

        if cmd.starts_with("remove-config") {
            // Format: remove-config
            wit_kv::remove(&room).context("writing to kv store")?;

            return Ok("removed config for that room!".to_owned());
        }

        if let Some(rest) = cmd.strip_prefix("allow") {
            // Format: allow USER_ID
            let mut split = rest.trim().split_whitespace();

            let user_id = split.next().context("missing user id")?;

            let mut current = wit_kv::get::<_, RoomConfig>(&room)
                .context("couldn't read room config for room")?
                .context("missing config for room")?;

            current.admins.push(user_id.to_owned());

            wit_kv::set(&room, &current).context("writing to kv store")?;

            return Ok("added admin!".to_owned());
        }

        if let Some(rest) = cmd.strip_prefix("disallow") {
            // Format: disallow USER_ID
            let mut split = rest.trim().split_whitespace();
            let user_id = split.next().context("missing user id")?;

            let mut current = wit_kv::get::<_, RoomConfig>(&room)
                .context("couldn't read room config for room")?
                .context("missing config for room")?;

            if let Some(idx) = current.admins.iter().position(|val| val == user_id) {
                current.admins.remove(idx);
            } else {
                return Ok("admin not found".to_owned());
            }

            wit_kv::set(&room, &current).context("writing to kv store")?;
            return Ok("removed admin!".to_owned());
        }

        if cmd.starts_with("list-posters") {
            // Format: list-posters ROOM
            let current = wit_kv::get::<_, RoomConfig>(&room)
                .context("couldn't read room config for room")?
                .context("no config for room")?;

            return Ok(current.admins.join(", "));
        }

        Ok("unknown command!".into())
    }
}

impl TrinityCommand for Component {
    fn init() {
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
        let Some(content) = content.strip_prefix("!toot").map(|rest| rest.trim()) else { return };
        let content = match Self::handle_msg(client.from(), &content, client.room()) {
            Ok(resp) => resp,
            Err(err) => err.to_string(),
        };
        client.respond(content);
    }

    fn on_admin(client: &mut CommandClient, cmd: &str) {
        let content = match Self::handle_admin(&cmd, client.from(), client.room()) {
            Ok(resp) => resp,
            Err(err) => err.to_string(),
        };
        client.respond(content);
    }
}

impl_command!(Component);
