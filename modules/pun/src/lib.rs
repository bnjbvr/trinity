use std::collections::HashMap;

use libcommand::*;
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

impl TrinityCommand for Component {
    fn init(_config: HashMap<String, String>) {
        let _ = log::set_boxed_logger(Box::new(log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn on_msg(client: &mut CommandClient, msg: &str) {
        if let Some(content) = Self::get_pun(msg) {
            client.respond(content);
        }
    }

    fn on_help(topic: Option<&str>) -> String {
        if topic == Some("toxic") {
            "this is content fetched from a website on the internet, so this may be toxic!"
        } else {
            "Get radioactive puns straight from the internet! (ask '!help pun toxic' for details on radioactivity)"
        }.to_owned()
    }
}

impl_command!();
