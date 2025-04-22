use std::collections::HashMap;

use libcommand::{impl_command, CommandClient, TrinityCommand};
use wit_log as log;
use wit_sync_request;

#[derive(serde::Serialize, serde::Deserialize)]
struct RoomConfig {
    base_url: String,
    token: String,
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
                "admin" | "!admin" => r#"available admin commands:
        - set-config #BASE_URL #TOKEN
        - remove-config"#
                    .into(),
                "sb" | "!sb" => "send a memo with the !sb TITLE CONTENT. The content can be multiline markdown, include tags, etc.".into(),
                _ => "i don't know this command!".into(),
            }
        } else {
            r#"Post memos to an instance of silverbullet.org from Matrix:

            1. First configure with `!admin silverbullet set-config`
            2. Then send memos to your instance with `!sb TITLE CONTENT
            3. ???
            4. Fun and profit!"#
                .to_owned()
        }
    }

    fn on_msg(client: &mut CommandClient, content: &str) {
        let Some(content) = content.strip_prefix("!sb").map(|rest| rest.trim()) else {
            return;
        };

        let mut split = content.split_whitespace();
        let Some(title) = split.next().filter(|title| !title.is_empty()) else {
            return client.respond("missing title!");
        };

        let content = content.strip_prefix(title).unwrap().trim();
        if content.is_empty() {
            return client.respond("no content to post!");
        }

        let room = client.room();

        let mut config = match wit_kv::get::<_, RoomConfig>(room) {
            Ok(Some(config)) => config,
            Ok(None) => return client.respond("missing room configuration"),
            Err(err) => {
                log::error!("error when reading configuration: {err}");
                return client.respond("error when reading configuration, check logs!");
            }
        };

        if !config.base_url.ends_with("/") {
            config.base_url.push('/');
        }

        let random = wit_sys::rand_u64();
        let url = format!("Inbox/{title}_{random}.md");
        log::trace!("about to send note to {url}");
        config.base_url.push_str(&url);

        client.respond(format!("posting content to: {}", config.base_url));

        let Ok(resp) = wit_sync_request::Request::put(&config.base_url)
            .header("Authorization", &format!("Bearer {}", config.token))
            .body(&content)
            .run()
        else {
            return client.respond("didn't receive a response from the server");
        };

        if resp.status != wit_sync_request::ResponseStatus::Success {
            log::info!(
                "request failed with non-success status code:\n\t{:?}",
                resp.body
            );
            return client.respond("error when sending memo, see logs!".to_owned());
        }

        client.react_with_ok();
    }

    fn on_admin(client: &mut CommandClient, cmd: &str) {
        let room = client.room();

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

        client.respond("unknown command!");
    }
}

impl_command!();
