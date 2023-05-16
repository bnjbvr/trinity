use libcommand::{impl_command, CommandClient};

struct Component;

impl libcommand::TrinityCommand for Component {
    fn on_help(_topic: Option<&str>) -> String {
        "Simple uuid generator".to_owned()
    }

    fn on_msg(client: &mut CommandClient, content: &str) {
        if !content.starts_with("!uuid") {
            return;
        }

        let r1 = wit_sys::rand_u64();
        let r2 = wit_sys::rand_u64();
        let uuid = uuid::Uuid::from_u64_pair(r1, r2);

        let content = format!("{uuid}");

        client.respond(content);
    }
}

impl_command!(Component);
