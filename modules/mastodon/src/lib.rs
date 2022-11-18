use anyhow::Context as _;
use bindings::interface;
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
    fn run(author_id: &str, content: &str, room: &str) -> anyhow::Result<String> {
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

    fn run_admin(cmd: &str, sender: &str) -> anyhow::Result<String> {
        if let Some(rest) = cmd.strip_prefix("set-config") {
            // Format: set-config ROOM BASE_URL TOKEN
            let mut split = rest.trim().split_whitespace();

            let room = split.next().context("missing room id")?;
            let base_url = split.next().context("missing base url")?;
            let token = split.next().context("missing token")?;

            let config = RoomConfig {
                admins: vec![sender.to_owned()],
                token: token.to_owned(),
                base_url: base_url.to_owned(),
            };

            wit_kv::set(room, &config).context("writing to kv store")?;
            return Ok("added!".to_owned());
        }

        if let Some(rest) = cmd.strip_prefix("allow") {
            // Format: allow ROOM USER_ID
            let mut split = rest.trim().split_whitespace();

            let room = split.next().context("missing room id")?;
            let user_id = split.next().context("missing user id")?;

            let mut current = wit_kv::get::<_, RoomConfig>(&room)
                .context("couldn't read room config for room")?
                .context("missing config for room")?;

            current.admins.push(user_id.to_owned());

            wit_kv::set(&room, &current).context("writing to kv store")?;
            return Ok("added!".to_owned());
        }

        if let Some(rest) = cmd.strip_prefix("disallow") {
            // Format: disallow ROOM USER_ID
            let mut split = rest.trim().split_whitespace();

            let room = split.next().context("missing room id")?;
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
            return Ok("removed!".to_owned());
        }

        if let Some(rest) = cmd.strip_prefix("list-posters") {
            // Format: list-posters ROOM
            let mut split = rest.trim().split_whitespace();

            let room = split.next().context("missing room id")?;

            let current = wit_kv::get::<_, RoomConfig>(&room)
                .context("couldn't read room config for room")?
                .context("no config for room")?;

            return Ok(current.admins.join(", "));
        }

        Ok("unknown command!".into())
    }
}

impl interface::Interface for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(crate::log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
    }

    fn help(topic: Option<String>) -> String {
        if let Some(topic) = topic {
            match topic.as_str() {
                "admin" => r#"available admin commands:
- set-config #ROOM_ID #BASE_URL #TOKEN
- allow #ROOM_ID #USER_ID
- disallow #ROOM_ID #USER_ID
- list-posters #ROOM_ID"#
                    .into(),
                "toot" | "!toot" => "Toot a message with !toot MESSAGE".into(),
                _ => "i don't know this command!".into(),
            }
        } else {
            "Post mastodon statuses from Matrix! Help topics: admin, toot".to_owned()
        }
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        room: String,
    ) -> Vec<interface::Message> {
        let Some(content) = content.strip_prefix("!toot").map(|rest| rest.trim()) else {
            return Vec::new();
        };
        let content = match Self::run(&author_id, &content, &room) {
            Ok(resp) => resp,
            Err(err) => err.to_string(),
        };
        vec![interface::Message {
            to: author_id,
            content,
        }]
    }

    fn admin(cmd: String, author: String) -> Vec<interface::Message> {
        let content = match Self::run_admin(&cmd, &author) {
            Ok(resp) => resp,
            Err(err) => err.to_string(),
        };

        vec![interface::Message {
            content,
            to: author,
        }]
    }
}

bindings::export!(Component);
