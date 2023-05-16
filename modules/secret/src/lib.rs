use libcommand::{impl_command, TrinityCommand};
use wit_log as log;

struct Component;

impl TrinityCommand for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn on_help(_topic: Option<&str>) -> String {
        "Secret tester".to_owned()
    }

    fn on_admin(client: &mut libcommand::CommandClient, cmd: &str) {
        let mut msg = None;

        match cmd.split_once(" ") {
            Some(("set", r)) => {
                if let Err(err) = wit_kv::set("secret", r) {
                    log::error!("ohnoes! error when setting the secret value: {err:#}");
                } else {
                    msg = Some("secret successfully set 👌".to_owned());
                }
            }

            _ => {
                if cmd == "get" {
                    let secret: Option<String> = wit_kv::get("secret").unwrap_or_else(|err| {
                        log::error!("couldn't read secret: {err:#}");
                        None
                    });
                    msg = Some(secret.unwrap_or_else(|| "<unset>".to_owned()));
                } else if cmd == "remove" {
                    if let Err(err) = wit_kv::remove("secret") {
                        log::error!("couldn't read value: {err:#}");
                    } else {
                        msg = Some("secret successfully unset 🤯".to_owned());
                    };
                } else {
                    msg = Some("i don't know this command??".to_owned());
                }
            }
        }

        if let Some(msg) = msg {
            client.respond(msg);
        }
    }

    fn on_msg(_client: &mut libcommand::CommandClient, _content: &str) {
        // Nothing!
    }
}

impl_command!(Component);
