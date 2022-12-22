use anyhow::Context as _;
use bindings::interface;
use wit_log as log;
use wit_sync_request;

#[derive(serde::Serialize, serde::Deserialize)]
struct RoomConfig {
    token: String,
}

struct Component;

const OPEN_AI_URL: &str = "https://api.openai.com/v1/completions";

impl Component {
    fn handle_msg(content: &str, room: &str) -> anyhow::Result<Option<String>> {
        let Some(config) = wit_kv::get::<_, RoomConfig>(room)? else { return Ok(None); };

        #[derive(serde::Serialize)]
        struct Request<'a> {
            model: &'a str,
            prompt: &'a str,
            max_tokens: u32,
            temperature: f64,
        }

        let body = serde_json::to_string(&Request {
            model: "text-davinci-003",
            prompt: content,
            max_tokens: 128,
            temperature: 0.1,
        })?;

        let resp = wit_sync_request::Request::post(OPEN_AI_URL)
            .header("Authorization", &format!("Bearer {}", config.token))
            .header("Content-Type", "application/json")
            .body(&body)
            .run()
            .ok()
            .context("no response")?;

        let resp_body = resp.body.context("missing response from OpenAI")?;

        log::trace!("received: {resp_body}");

        #[allow(unused)]
        #[derive(serde::Deserialize)]
        struct OpenAiChoice {
            text: String,
            index: u32,
            log_probes: Option<()>,
            finish_reason: String,
        }

        #[derive(serde::Deserialize)]
        struct OpenAiResponse {
            choices: Vec<OpenAiChoice>,
        }

        let resp: OpenAiResponse = serde_json::from_str(&resp_body)?;

        if let Some(first_choice) = resp.choices.first() {
            Ok(Some(first_choice.text.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    fn handle_admin(cmd: &str, room: &str) -> anyhow::Result<String> {
        if let Some(rest) = cmd.strip_prefix("enable") {
            // Format: set-config TOKEN
            let token = rest.trim();
            let config = RoomConfig {
                token: token.to_owned(),
            };
            wit_kv::set(&room, &config).context("writing to kv store")?;
            return Ok("added!".to_owned());
        }

        if cmd.starts_with("disable") {
            // Format: remove-config
            wit_kv::remove(&room).context("writing to kv store")?;
            return Ok("removed config for that room!".to_owned());
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
- enable #TOKEN
- disable"#
                    .into(),
                _ => "i don't know this command!".into(),
            }
        } else {
            "Chat using OpenAI! Will respond to every message given it's configured in a room. Help topics: admin".to_owned()
        }
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        room: String,
    ) -> Vec<interface::Message> {
        let content = match Self::handle_msg(&content, &room) {
            Ok(Some(resp)) => resp,
            Ok(None) => return Vec::new(),
            Err(err) => err.to_string(),
        };
        vec![interface::Message {
            content,
            to: author_id,
        }]
    }

    fn admin(cmd: String, author: String, room: String) -> Vec<interface::Message> {
        let content = match Self::handle_admin(&cmd, &room) {
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
