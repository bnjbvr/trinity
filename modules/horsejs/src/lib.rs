use bindings::interface;
use wit_log as log;
use wit_sync_request;

struct Component;

impl Component {
    fn get_quote(msg: &str) -> Option<String> {
        if !msg.starts_with("!horsejs") {
            return None;
        }

        const URL: &str = "https://javascript.horse/random.json";

        let resp = wit_sync_request::Request::get(URL)
            .header("Accept", "application/json")
            .run()
            .ok()?;

        if resp.status != wit_sync_request::ResponseStatus::Success {
            log::info!("request failed with non-success status code");
        }

        #[derive(serde::Deserialize)]
        struct Response {
            text: String,
        }

        serde_json::from_str::<Response>(&resp.body?)
            .ok()
            .map(|resp| resp.text)
    }
}

impl interface::Interface for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(crate::log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn help() -> String {
        "Contextless twitter quotes about the JavaScript".to_owned()
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<interface::Message> {
        if let Some(content) = Self::get_quote(&content) {
            vec![interface::Message {
                content,
                to: author_id,
            }]
        } else {
            vec![]
        }
    }
}

bindings::export!(Component);
