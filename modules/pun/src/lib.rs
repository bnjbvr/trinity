use bindings::interface;

use wit_log as log;
use wit_sync_request;

struct Component;

impl Component {
    fn get_pun(msg: &str) -> Option<String> {
        if !msg.starts_with("!pun") {
            return None;
        }

        const URL: &str = "https://icanhazdadjoke.com/";

        let resp = wit_sync_request::Request::get(URL)
            .header("Accept", "application/json")
            .run()
            .ok()?;

        if resp.status != wit_sync_request::ResponseStatus::Success {
            log::info!("request failed with non-success status code");
        }

        #[derive(serde::Deserialize)]
        struct Response {
            joke: String,
        }

        serde_json::from_str::<Response>(&resp.body?)
            .ok()
            .map(|resp| resp.joke)
    }
}

impl interface::Interface for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn help(topic: Option<String>) -> String {
        if let Some(topic) = topic {
            if topic == "toxic" {
                return "this is content fetched from a website on the internet, so this may be toxic!".to_owned();
            }
        }
        "Get radioactive puns straight from the internet! (ask '!help pun toxic' for details on radioactivity)".to_owned()
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<interface::Message> {
        if let Some(content) = Self::get_pun(&content) {
            vec![interface::Message {
                content,
                to: author_id,
            }]
        } else {
            vec![]
        }
    }

    fn admin(_cmd: String, _author: String, _room: String) -> Vec<interface::Message> {
        Vec::new()
    }
}

bindings::export!(Component);
