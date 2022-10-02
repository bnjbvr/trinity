wit_bindgen_guest_rust::export!("../../wit/exports.wit");

use wit_log;
use wit_sync_request;

struct Exports;

impl Exports {
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

impl exports::Exports for Exports {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(crate::wit_log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn help() -> String {
        "Get radioactive puns straight from the internet!".to_owned()
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<exports::Message> {
        if let Some(content) = Self::get_pun(&content) {
            vec![exports::Message {
                content,
                to: author_id,
            }]
        } else {
            vec![]
        }
    }
}
