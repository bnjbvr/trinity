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

/// Implements a command for a given type, assuming the type implements the `TrinityCommand` trait.
#[macro_export]
macro_rules! impl_command {
    ($ident:ident) => {
        const _: () = {
            fn consume_client(client: $crate::CommandClient) -> Vec<bindings::messaging::Action> {
                let mut actions = Vec::new();

                actions.extend(client.messages.into_iter().map(|msg| {
                    bindings::messaging::Action::Respond(bindings::messaging::Message {
                        text: msg.1,
                        html: None,
                        to: msg.0 .0,
                    })
                }));

                actions.extend(
                    client
                        .reactions
                        .into_iter()
                        .map(|reaction| bindings::messaging::Action::React(reaction)),
                );

                actions
            }

            impl bindings::messaging::Messaging for $ident {
                fn init() {
                    <Self as $crate::TrinityCommand>::init();
                }

                fn help(topic: Option<String>) -> String {
                    <Self as $crate::TrinityCommand>::on_help(topic.as_deref())
                }

                fn on_msg(
                    content: String,
                    author_id: String,
                    _author_name: String,
                    room: String,
                ) -> Vec<bindings::messaging::Action> {
                    let mut client = $crate::CommandClient::new(room, author_id.clone());
                    <Self as $crate::TrinityCommand>::on_msg(&mut client, &content);
                    consume_client(client)
                }

                fn admin(
                    cmd: String,
                    author_id: String,
                    room: String,
                ) -> Vec<bindings::messaging::Action> {
                    let mut client = $crate::CommandClient::new(room.clone(), author_id);
                    <Self as $crate::TrinityCommand>::on_admin(&mut client, &cmd);
                    consume_client(client)
                }
            }

            bindings::export!($ident);
        };
    };
}

pub struct Recipient(pub String);

pub struct CommandClient {
    inbound_msg_room: String,
    inbound_msg_author: String,
    pub messages: Vec<(Recipient, String)>,
    pub reactions: Vec<String>,
}

impl CommandClient {
    pub fn new(room: String, author: String) -> Self {
        Self {
            inbound_msg_room: room,
            inbound_msg_author: author,
            messages: Default::default(),
            reactions: Default::default(),
        }
    }

    /// Who sent the original message we're reacting to?
    pub fn from(&self) -> &str {
        &self.inbound_msg_author
    }

    /// Indicates in which room this message has been received.
    pub fn room(&self) -> &str {
        &self.inbound_msg_room
    }

    /// Queues a message to be sent to the author of the original message.
    pub fn respond(&mut self, msg: impl Into<String>) {
        self.respond_to(msg.into(), self.inbound_msg_author.clone())
    }

    /// Queues a message to be sent to someone else.
    pub fn respond_to(&mut self, msg: String, author: String) {
        self.messages.push((Recipient(author), msg));
    }

    pub fn react_with(&mut self, reaction: String) {
        self.reactions.push(reaction);
    }

    pub fn react_with_ok(&mut self) {
        self.react_with("👌".to_owned());
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
    fn on_admin(_client: &mut CommandClient, _command: &str) {}
}
