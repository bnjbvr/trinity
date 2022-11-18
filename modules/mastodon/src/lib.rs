use anyhow::Context as _;
use bindings::interface;
use wit_log as log;
use wit_sync_request;

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Credentials {
    admins: Vec<String>,
}

struct Component;

impl Component {
    fn run(author_id: &str, content: &str) -> anyhow::Result<String> {
        let credentials = wit_kv::get::<_, Credentials>("credentials")
            .context("couldn't read mastodon credentials")?
            .context("missing credentials")?;

        anyhow::ensure!(
            credentials.admins.iter().any(|admin| *admin == author_id),
            "you're not allowed to post, sorry!"
        );

        let token = wit_kv::get::<_, String>("token")
            .context("couldn't read mastodon token")?
            .context("missing token")?;

        let mut base_url = wit_kv::get::<_, String>("base_url")
            .context("couldn't read mastodon base url")?
            .context("missing base_url!")?;

        if !base_url.ends_with("/") {
            base_url.push('/');
        }
        base_url.push_str("api/v1/statuses");

        #[derive(serde::Serialize)]
        struct Request {
            status: String,
        }

        let body = serde_json::to_string(&Request {
            status: content.to_owned(),
        })?;

        let resp = wit_sync_request::Request::post(&base_url)
            .header("Authorization", &format!("Bearer {token}"))
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
}

impl interface::Interface for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(crate::log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
    }

    fn help(topic: Option<String>) -> String {
        if let Some(topic) = topic {
            match topic.as_str() {
                "admin" => {
                    "available admin commands: allow #USER_ID/disallow #USER_ID/list-posters/set-base-url/set-token".into()
                }
                _ => {
                    "i don't know this command!".into()
                }
            }
        } else {
            "Post mastodon statuses from Matrix!".to_owned()
        }
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<interface::Message> {
        let Some(content) = content.strip_prefix("!toot").map(|rest| rest.trim()) else {
            return Vec::new();
        };
        let content = match Self::run(&author_id, &content) {
            Ok(resp) => resp,
            Err(err) => err.to_string(),
        };
        vec![interface::Message {
            to: author_id,
            content,
        }]
    }

    fn admin(cmd: String, author: String) -> Vec<interface::Message> {
        let msg: String = if let Some(token) = cmd.strip_prefix("set-token") {
            let _ = wit_kv::set("token", token.trim());
            "token successfully set!".into()
        } else if let Some(base_url) = cmd.strip_prefix("set-base-url") {
            let _ = wit_kv::set("base_url", base_url.trim());
            "base url successfully set!".into()
        } else if cmd == "remove" {
            let _ = wit_kv::remove("token");
            "token successfully unset!".into()
        } else if let Some(user_id) = cmd.strip_prefix("allow") {
            let user_id = user_id.trim();
            if let Ok(prev) = wit_kv::get::<_, Credentials>("credentials") {
                let mut prev = prev.unwrap_or_default();
                prev.admins.push(user_id.to_string());
                let _ = wit_kv::set("credentials", &prev);
            }
            "admin successfully added!".into()
        } else if let Some(user_id) = cmd.strip_prefix("disallow") {
            let user_id = user_id.trim();
            if let Ok(prev) = wit_kv::get::<_, Credentials>("credentials") {
                let mut prev = prev.unwrap_or_default();
                prev.admins.retain(|admin| admin != user_id);
                let _ = wit_kv::set("credentials", &prev);
            }
            "admin successfully removed!".into()
        } else if cmd == "list-posters" {
            if let Ok(prev) = wit_kv::get::<_, Credentials>("credentials") {
                let prev = prev.unwrap_or_default();
                format!("posters: {}", prev.admins.join(", "))
            } else {
                "error when reading admins".into()
            }
        } else {
            "i don't know this command".into()
        };

        vec![interface::Message {
            content: msg.to_string(),
            to: author,
        }]
    }
}

bindings::export!(Component);
