wit_bindgen_guest_rust::import!("../../wit/imports.wit");
wit_bindgen_guest_rust::export!("../../wit/exports.wit");

struct Exports;

impl exports::Exports for Exports {
    fn init() {}

    fn help() -> String {
        "Simple uuid generator".to_owned()
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<exports::Message> {
        if !content.starts_with("!uuid") {
            return vec![];
        }

        let r1 = imports::rand_u64();
        let r2 = imports::rand_u64();
        let uuid = uuid::Uuid::from_u64_pair(r1, r2);

        let content = format!("{uuid}");

        vec![exports::Message {
            content,
            to: author_id,
        }]
    }
}
