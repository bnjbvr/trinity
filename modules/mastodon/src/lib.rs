use bindings::interface;
use wit_log as log;
use wit_sync_request;

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Credentials {
    admins: Vec<String>,
}

struct Component;

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
        if let Some(content) = content.strip_prefix("!toot").map(|rest| rest.trim()) {
            let Ok(credentials): Result<Option<Credentials>, _> = wit_kv::get("credentials") else {
                log::warn!("couldn't read mastodon credentials");
                return Vec::new();
            };

            let Some(credentials) = credentials else {
                return vec!(interface::Message {
                    content: "missing credentials!".to_owned(),
                    to: author_id,
                });
            };

            for admin in &credentials.admins {
                log::warn!("{author_id} == {admin}? {:?}", author_id == *admin);
            }

            if credentials.admins.iter().all(|admin| *admin != author_id) {
                return vec![interface::Message {
                    content: "you're not authorized to post!".to_owned(),
                    to: author_id,
                }];
            }

            let Ok(token): Result<Option<String>, _> = wit_kv::get("token") else {
                log::warn!("couldn't read mastodon token");
                return Vec::new();
            };

            let Some(token) = token else {
                return vec!(interface::Message {
                    content: "missing token!".to_owned(),
                    to: author_id,
                });
            };

            let Ok(base_url): Result<Option<String>, _> = wit_kv::get("base_url") else {
                log::warn!("couldn't read mastodon base url");
                return Vec::new();
            };

            let Some(mut base_url) = base_url else {
                return vec!(interface::Message {
                    content: "missing base_url!".to_owned(),
                    to: author_id,
                });
            };

            if !base_url.ends_with("/") {
                base_url.push('/');
            }
            base_url.push_str("api/v1/statuses");

            #[derive(serde::Serialize)]
            struct Request {
                status: String,
            }

            let Ok(body) = serde_json::to_string(&Request {
                    status: content.to_owned(),
                }) else {
                log::error!("error when serializing mastodon request");
                return Vec::new();
            };

            let Some(resp) = wit_sync_request::Request::post(&base_url)
                .header("Authorization", &format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(&body)
                .run()
                .ok() else {
                return vec!(interface::Message {
                    content: "no response".to_owned(),
                    to: author_id,
                });
            };

            if resp.status != wit_sync_request::ResponseStatus::Success {
                log::info!(
                    "request failed with non-success status code:\n\t{:?}",
                    resp.body
                );
                return vec![interface::Message {
                    content: "error when sending toot, see logs!".to_owned(),
                    to: author_id,
                }];
            }

            return vec![interface::Message {
                content: "great success!".to_owned(),
                to: author_id,
            }];
        }

        Vec::new()
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
