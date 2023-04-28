//! High-level library providing a trait that, once implemented, hides the complexity of
//! Wit bindings.
//!
//! There are a few problems at the moment:
//!
//! - It's not possible for a lib to implement a component interface, as it has to be the final
//! binary implementing the component's interface; see also
//! https://github.com/bytecodealliance/cargo-component/issues/75.
//!
//! - Because of that, I've had to put most of the code, including the whole `impl Interface for X`
//! block, in the macro body. It's ugly and not practical for maintainability purposes.

#[macro_export]
macro_rules! impl_command {
    ($ident:ident) => {
        const _: () = {
            type Wrapped = TrinityCommandWrapper<Component>;

            /// Small wrapper which sole purpose is to work around the impossibility to have `impl Interface
            /// for T where T: TrinityCommand`.
            #[doc(hidden)]
            pub struct TrinityCommandWrapper<T> {
                _phantom: std::marker::PhantomData<T>,
            }

            impl<T> bindings::interface::Interface for TrinityCommandWrapper<T>
            where
                T: TrinityCommand,
            {
                fn init() {
                    <T as TrinityCommand>::init();
                }

                fn help(topic: Option<String>) -> String {
                    <T as TrinityCommand>::on_help(topic.as_deref())
                }

                fn on_msg(
                    content: String,
                    author_id: String,
                    _author_name: String,
                    _room: String,
                ) -> Vec<bindings::interface::Message> {
                    let mut client = CommandClient::default();
                    <T as TrinityCommand>::on_msg(&mut client, &content);
                    client
                        .messages
                        .into_iter()
                        .map(|msg| bindings::interface::Message {
                            content: msg,
                            to: author_id.clone(),
                        })
                        .collect()
                }

                fn admin(
                    cmd: String,
                    author_id: String,
                    room: String,
                ) -> Vec<bindings::interface::Message> {
                    let mut client = CommandClient::default();
                    <T as TrinityCommand>::on_admin(&mut client, &cmd, &room);
                    client
                        .messages
                        .into_iter()
                        .map(|msg| bindings::interface::Message {
                            content: msg,
                            to: author_id.clone(),
                        })
                        .collect()
                }
            }

            bindings::export!(Wrapped);
        };
    };
}

#[derive(Default)]
pub struct CommandClient {
    pub messages: Vec<String>,
}

impl CommandClient {
    /// Queues a message to be sent to someone.
    pub fn respond(&mut self, msg: String) {
        self.messages.push(msg);
    }
}

pub trait TrinityCommand {
    /// Code that will be called once during initialization of the command. This is a good time to
    /// retrieve settings from the database and cache them locally, if needs be, or run any
    /// initialization code that shouldn't run on every message later.
    fn init() {}

    /// Handle a message received in a room where the bot is present.
    ///
    /// The message isn't identified as a request for help or an admin command. Those are handled
    /// respectively by `on_help` and `on_admin`.
    ///
    /// This should always be implemented, otherwise the command doesn't do anything.
    fn on_msg(client: &mut CommandClient, content: &str);

    /// Respond to a help request, for this specific command.
    ///
    /// If the topic is not set, then this should return a general description of the command, with
    /// hints to the possible topics. If the topic is set, then this function should document
    /// something related to the specific topic.
    ///
    /// This should always be implemented, at least to document what's the command's purpose.
    fn on_help(_topic: Option<&str>) -> String;

    /// Handle a message received by an admin, prefixed with the `!admin` subject.
    ///
    /// By default this does nothing, as admin commands are facultative.
    fn on_admin(_client: &mut CommandClient, _command: &str, _room: &str) {}
}
