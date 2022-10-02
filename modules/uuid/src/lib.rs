wit_bindgen_guest_rust::import!("../../wit/imports.wit");
wit_bindgen_guest_rust::export!("../../wit/exports.wit");

use wit_log;

struct Exports;

impl exports::Exports for Exports {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(crate::wit_log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn help() -> String {
        "Simple uuid generator".to_owned()
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<exports::Message> {
        log::trace!("hello from wasm module!");

        if !content.starts_with("!uuid") {
            log::trace!("message '{}' doesn't start with !uuid", content);
            return vec![];
        }

        let r1 = imports::rand_u64();
        let r2 = imports::rand_u64();
        let uuid = uuid::Uuid::from_u64_pair(r1, r2);

        let content = format!("{uuid}");

        log::trace!("definitely returning a message now!");
        vec![exports::Message {
            content,
            to: author_id,
        }]
    }
}
