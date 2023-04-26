use bindings::interface;

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

// TODO experiment further with that
enum Msg<'a> {
    Admin { command: &'a str },
    Help { topic: Option<&'a str> },
    Message { content: &'a str },
}

struct MsgMetadata {
    is_help: bool,
    is_admin: bool,
}

#[derive(Default)]
struct Messenger {
    msg: Option<String>,
}

impl Messenger {
    fn respond(&mut self, msg: String) -> anyhow::Result<()> {
        let prev = self.msg.replace(msg);
        anyhow::ensure!(
            prev.is_none(),
            "already set a message, multiple messages NYI"
        );
        Ok(())
    }
}

trait TrinityCommand {
    fn init() {}
    fn on_msg(client: &mut Messenger, content: &str, metadata: MsgMetadata);
}

impl TrinityCommand for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn on_msg(client: &mut Messenger, content: &str, metadata: MsgMetadata) {
        if metadata.is_help {
            if let Some(topic) = content {
                if topic == "toxic" {
                    client.respond("this is content fetched from a website on the internet, so this may be toxic!".to_owned());
                }
            }
            client.respond("Get radioactive puns straight from the internet! (ask '!help pun toxic' for details on radioactivity)".to_owned());
        } else if metadata.is_admin {
            client.respond("I don't have any admin commands".to_owned());
        } else if let Some(content) = Self::get_pun(content) {
            client.respond(content);
        }
    }
}

impl<T> interface::Interface for T where T: TrinityCommand
{
    fn init() {
        <Self as TrinityCommand>::init();
    }

    fn help(topic: Option<String>) -> String {
        let mut client = Messenger::default();
        <Self as TrinityCommand>::on_msg(
            &mut client,
            topic.as_ref().unwrap_or(""),
            MsgMetadata {
                is_help: true,
                is_admin: false,
            },
        );
        client
            .msg
            .unwrap_or_else(|| String::from("<no help specified by the module>"))
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<interface::Message> {
        let mut client = Messenger::default();
        <Self as TrinityCommand>::on_msg(
            &mut client,
            &content,
            MsgMetadata {
                is_help: false,
                is_admin: false,
            },
        );
        if let Some(response) = client.msg {
            vec![interface::Message {
                content: response,
                to: author_id,
            }]
        } else {
            vec![]
        }
    }

    fn admin(cmd: String, author: String, _room: String) -> Vec<interface::Message> {
        let mut client = Messenger::default();
        <Self as TrinityCommand>::on_msg(
            &mut client,
            &cmd,
            MsgMetadata {
                is_help: false,
                is_admin: true,
            },
        );
        if let Some(response) = client.msg {
            vec![interface::Message {
                content: response,
                to: author,
            }]
        } else {
            vec![]
        }
    }
}

bindings::export!(Component);
