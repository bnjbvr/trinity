use bindings::interface;

struct Component;

impl interface::Interface for Component {
    fn init() {}

    fn help(_topic: Option<String>) -> String {
        "Simple uuid generator".to_owned()
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<interface::Message> {
        if !content.starts_with("!uuid") {
            return vec![];
        }

        let r1 = wit_sys::rand_u64();
        let r2 = wit_sys::rand_u64();
        let uuid = uuid::Uuid::from_u64_pair(r1, r2);

        let content = format!("{uuid}");

        vec![interface::Message {
            content,
            to: author_id,
        }]
    }
}

bindings::export!(Component);
