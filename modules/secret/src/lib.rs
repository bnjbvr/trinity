use bindings::interface;

#[derive(Default)]
struct Context {
    secret: Option<String>,
}

static mut CTX: Option<Context> = None;

struct Component;

impl interface::Interface for Component {
    fn init() {}

    fn help(_topic: Option<String>) -> String {
        "Secret tester".to_owned()
    }

    fn on_msg(
        _content: String,
        _author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<interface::Message> {
        Vec::new()
    }

    fn admin(cmd: String, author_id: String) -> Vec<interface::Message> {
        let ctx = unsafe { CTX.get_or_insert_with(Default::default) };

        match cmd.split_once(" ") {
            Some(("set", r)) => {
                ctx.secret = Some(r.to_owned());
                return vec![interface::Message {
                    content: "secret successfully set 👌".to_owned(),
                    to: author_id,
                }];
            }
            _ => {
                if cmd == "get" {
                    return vec![interface::Message {
                        content: ctx.secret.clone().unwrap_or_else(|| "<unset>".to_owned()),
                        to: author_id,
                    }];
                }
            }
        }

        Vec::new()
    }
}

bindings::export!(Component);
