use bindings::interface;

use wit_log as log;
use wit_sync_request;

// TODO move to high-level library
macro_rules! impl_command {
    ($ident:ident) => {
        type Wrapped = TrinityCommandWrapper<Component>;
        bindings::export!(Wrapped);
    };
}

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

impl TrinityCommand for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn on_msg(client: &mut Messenger, msg: Msg<'_>) -> anyhow::Result<()> {
        match msg {
            Msg::Admin { command: _ } => {
                client.respond("I don't have any admin commands".to_owned())?;
            }
            Msg::Help { topic } => {
                if topic == Some("toxic") {
                    client.respond(
                    "this is content fetched from a website on the internet, so this may be toxic!"
                        .to_owned(),
                )?;
                } else {
                    client.respond("Get radioactive puns straight from the internet! (ask '!help pun toxic' for details on radioactivity)".to_owned())?;
                }
            }
            Msg::Message { content } => {
                if let Some(content) = Self::get_pun(content) {
                    client.respond(content)?;
                }
            }
        }
        Ok(())
    }
}

impl_command!(Component);

// FRAMEWORK BITS, TODO move out to a shared library

enum Msg<'a> {
    #[allow(unused)]
    Admin {
        command: &'a str,
    },
    Help {
        topic: Option<&'a str>,
    },
    Message {
        content: &'a str,
    },
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
    fn on_msg(client: &mut Messenger, content: Msg<'_>) -> anyhow::Result<()>;
}

/// Small wrapper which sole purpose is to work around the impossibility to have `impl Interface
/// for T where T: TrinityCommand`.
struct TrinityCommandWrapper<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> interface::Interface for TrinityCommandWrapper<T>
where
    T: TrinityCommand,
{
    fn init() {
        <T as TrinityCommand>::init();
    }

    fn help(topic: Option<String>) -> String {
        let mut client = Messenger::default();
        match <T as TrinityCommand>::on_msg(
            &mut client,
            Msg::Help {
                topic: topic.as_deref(),
            },
        ) {
            Ok(()) => client
                .msg
                .unwrap_or_else(|| String::from("<no help specified by the module>")),
            Err(err) => format!("<error when handling help from module: {err:#}>"),
        }
    }

    fn on_msg(
        content: String,
        author_id: String,
        _author_name: String,
        _room: String,
    ) -> Vec<interface::Message> {
        let mut client = Messenger::default();
        match <T as TrinityCommand>::on_msg(&mut client, Msg::Message { content: &content }) {
            Ok(()) => {
                if let Some(response) = client.msg {
                    vec![interface::Message {
                        content: response,
                        to: author_id,
                    }]
                } else {
                    vec![]
                }
            }
            Err(err) => {
                vec![interface::Message {
                    content: format!("<error when handling message from module: {err:#}>"),
                    to: author_id,
                }]
            }
        }
    }

    fn admin(cmd: String, author_id: String, _room: String) -> Vec<interface::Message> {
        let mut client = Messenger::default();
        match <T as TrinityCommand>::on_msg(&mut client, Msg::Admin { command: &cmd }) {
            Ok(()) => {
                if let Some(response) = client.msg {
                    vec![interface::Message {
                        content: response,
                        to: author_id,
                    }]
                } else {
                    vec![]
                }
            }
            Err(err) => {
                vec![interface::Message {
                    content: format!("<error when handling admin from module: {err:#}>"),
                    to: author_id,
                }]
            }
        }
    }
}
