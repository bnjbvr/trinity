use std::collections::HashMap;

use libcommand::{impl_command, TrinityCommand};
use wit_log as log;

struct Component;

impl TrinityCommand for Component {
    fn init(_config: HashMap<String, String>) {
        let _ = log::set_boxed_logger(Box::new(log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn on_help(_topic: Option<&str>) -> String {
        "Secret tester".to_owned()
    }

    fn on_admin(client: &mut libcommand::CommandClient, cmd: &str) {
        match cmd.split_once(" ") {
            Some(("set", r)) => {
                if let Err(err) = wit_kv::set("secret", r) {
                    log::error!("ohnoes! error when setting the secret value: {err:#}");
                } else {
                    client.react_with("👌".to_owned());
                }
            }

            _ => {
                if cmd == "get" {
                    let secret: Option<String> = wit_kv::get("secret").unwrap_or_else(|err| {
                        log::error!("couldn't read secret: {err:#}");
                        None
                    });
                    client.respond(secret.unwrap_or_else(|| "<unset>".to_owned()));
                } else if cmd == "remove" {
                    if let Err(err) = wit_kv::remove("secret") {
                        log::error!("couldn't read value: {err:#}");
                    } else {
                        client.react_with("🤯".to_owned());
                    };
                } else {
                    client.respond("i don't know this command??".to_owned());
                }
            }
        }
    }

    fn on_msg(_client: &mut libcommand::CommandClient, _content: &str) {
        // Nothing!
    }
}

impl_command!(Component);
