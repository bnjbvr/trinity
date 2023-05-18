mod admin_table;
mod room_resolver;
mod wasm;

use anyhow::Context;
use matrix_sdk::{
    config::SyncSettings,
    event_handler::Ctx,
    room::Room,
    ruma::{
        events::{
            reaction::{ReactionEventContent, Relation},
            room::{
                member::StrippedRoomMemberEvent,
                message::{MessageType, RoomMessageEventContent, SyncRoomMessageEvent},
            },
        },
        presence::PresenceState,
        OwnedUserId, RoomId, UserId,
    },
    Client,
};
use notify::{RecursiveMode, Watcher};
use room_resolver::RoomResolver;
use std::{env, path::PathBuf, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};
use wasm::{GuestState, Module, WasmModules};

use crate::admin_table::DEVICE_ID_ENTRY;

/// The configuration to run a trinity instance with.
pub struct BotConfig {
    /// the matrix homeserver the bot should connect to.
    pub home_server: String,
    /// the user_id to be used on the homeserver.
    pub user_id: String,
    /// password to be used to log into the homeserver.
    pub password: String,
    /// where to store the matrix-sdk internal data.
    pub matrix_store_path: String,
    /// where to store the additional database data.
    pub redb_path: String,
    /// the admin user id for the bot.
    pub admin_user_id: OwnedUserId,
    /// paths where modules can be loaded.
    pub modules_paths: Vec<PathBuf>,
}

impl BotConfig {
    /// Generate a `BotConfig` from the process' environment.
    pub fn from_env() -> anyhow::Result<Self> {
        // override environment variables with contents of .env file, unless they were already set
        // explicitly.
        dotenvy::dotenv().ok();

        let home_server = env::var("HOMESERVER").context("missing HOMESERVER variable")?;
        let user_id = env::var("BOT_USER_ID").context("missing bot user id in BOT_USER_ID")?;
        let password = env::var("BOT_PWD").context("missing bot user id in BOT_PWD")?;
        let matrix_store_path =
            env::var("MATRIX_STORE_PATH").context("missing MATRIX_STORE_PATH")?;
        let redb_path = env::var("REDB_PATH").context("missing REDB_PATH")?;

        let admin_user_id =
            env::var("ADMIN_USER_ID").context("missing admin user id in ADMIN_USER_ID")?;
        let admin_user_id = admin_user_id
            .try_into()
            .context("impossible to parse admin user id")?;

        // Read the module paths (separated by commas), check they exist, and return the whole
        // list.
        let modules_paths = env::var("MODULES_PATHS")
            .as_deref()
            .unwrap_or("./modules/target/wasm32-unknown-unknown/release")
            .split(',')
            .map(|path| {
                let path = PathBuf::from(path);
                anyhow::ensure!(
                    path.exists(),
                    "{} doesn't reference a valid path",
                    path.to_string_lossy()
                );
                Ok(path)
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .context("a module path isn't valid")?;

        Ok(Self {
            home_server,
            user_id,
            password,
            matrix_store_path,
            admin_user_id,
            redb_path,
            modules_paths,
        })
    }
}

pub(crate) type ShareableDatabase = Arc<redb::Database>;

struct AppCtx {
    modules: WasmModules,
    modules_paths: Vec<PathBuf>,
    needs_recompile: bool,
    admin_user_id: OwnedUserId,
    db: ShareableDatabase,
    room_resolver: RoomResolver,
}

impl AppCtx {
    /// Create a new `AppCtx`.
    ///
    /// Must be called from a blocking context.
    pub fn new(
        client: Client,
        modules_paths: Vec<PathBuf>,
        db: ShareableDatabase,
        admin_user_id: OwnedUserId,
    ) -> anyhow::Result<Self> {
        let room_resolver = RoomResolver::new(client);
        Ok(Self {
            modules: WasmModules::new(db.clone(), &modules_paths)?,
            modules_paths,
            needs_recompile: false,
            admin_user_id,
            db,
            room_resolver,
        })
    }

    pub async fn set_needs_recompile(ptr: Arc<Mutex<Self>>) {
        {
            let need = &mut ptr.lock().await.needs_recompile;
            if *need {
                return;
            }
            *need = true;
        }

        tokio::task::spawn_blocking(move || {
            let mut ptr = futures::executor::block_on(async {
                tokio::time::sleep(Duration::new(1, 0)).await;
                ptr.lock().await
            });

            match WasmModules::new(ptr.db.clone(), &ptr.modules_paths) {
                Ok(modules) => {
                    ptr.modules = modules;
                    tracing::info!("successful hot reload!");
                }
                Err(err) => {
                    tracing::error!("hot reload failed: {err:#}");
                }
            }

            ptr.needs_recompile = false;
        });
    }
}

#[derive(Clone)]
struct App {
    inner: Arc<Mutex<AppCtx>>,
}

impl App {
    pub fn new(ctx: AppCtx) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ctx)),
        }
    }
}

/// Try to handle a message assuming it's an `!admin` command.
fn try_handle_admin<'a>(
    content: &str,
    sender: &UserId,
    room: &RoomId,
    store: &mut wasmtime::Store<GuestState>,
    modules: impl Clone + Iterator<Item = &'a Module>,
    room_resolver: &mut RoomResolver,
) -> Option<Vec<wasm::Action>> {
    let Some(rest) = content.strip_prefix("!admin") else { return None };

    tracing::trace!("trying admin for {content}");

    if let Some(rest) = rest.strip_prefix(' ') {
        let rest = rest.trim();
        if let Some((module, rest)) = rest.split_once(' ').map(|(l, r)| (l, r.trim())) {
            // If the next word resolves to a valid room id use that, otherwise use the
            // current room.
            let (possible_room, rest) = rest
                .split_once(' ')
                .map_or((rest, ""), |(l, r)| (l, r.trim()));

            let (target_room, rest) = match room_resolver.resolve_room(possible_room) {
                Ok(Some(resolved_room)) => (resolved_room, rest.to_string()),
                Ok(None) | Err(_) => (room.to_string(), format!("{} {}", possible_room, rest)),
            };

            let mut found = None;
            for m in modules {
                if m.name() == module {
                    found = match m.admin(&mut *store, rest.trim(), sender, target_room.as_str()) {
                        Ok(actions) => Some(actions),
                        Err(err) => {
                            tracing::error!("error when handling admin command: {err:#}");
                            None
                        }
                    };
                    break;
                }
            }
            found
        } else {
            Some(vec![wasm::Action::Respond(wasm::Message {
                text: "missing command".to_owned(),
                html: None,
                to: sender.to_string(),
            })])
        }
    } else {
        Some(vec![wasm::Action::Respond(wasm::Message {
            text: "missing module and command".to_owned(),
            html: None,
            to: sender.to_string(),
        })])
    }
}

fn try_handle_help<'a>(
    content: &str,
    sender: &UserId,
    store: &mut wasmtime::Store<GuestState>,
    modules: impl Clone + Iterator<Item = &'a Module>,
) -> Option<wasm::Action> {
    let Some(rest) = content.strip_prefix("!help") else { return None };

    // Special handling for help messages.
    let (msg, html) = if rest.trim().is_empty() {
        let mut msg = String::from("Available modules:");
        let mut html = String::from("Available modules: <ul>");
        for m in modules {
            let help = match m.help(&mut *store, None) {
                Ok(msg) => Some(msg),
                Err(err) => {
                    tracing::error!("error when handling help command: {err:#}");
                    None
                }
            }
            .unwrap_or("<missing>".to_string());

            msg.push_str(&format!("\n- {name}: {help}", name = m.name(), help = help));
            // TODO lol sanitize html
            html.push_str(&format!(
                "<li><b>{name}</b>: {help}</li>",
                name = m.name(),
                help = help
            ));
        }
        html.push_str("</ul>");

        (msg, html)
    } else if let Some(rest) = rest.strip_prefix(' ') {
        let rest = rest.trim();
        let (module, topic) = rest
            .split_once(' ')
            .map(|(l, r)| (l, Some(r.trim())))
            .unwrap_or((rest, None));
        let mut found = None;
        for m in modules {
            if m.name() == module {
                found = m.help(&mut *store, topic).ok();
                break;
            }
        }
        let msg = if let Some(content) = found {
            content
        } else {
            format!("module {module} not found")
        };
        (msg.clone(), msg)
    } else {
        return None;
    };

    return Some(wasm::Action::Respond(wasm::Message {
        text: msg,
        html: Some(html),
        to: sender.to_string(), // TODO rather room?
    }));
}

enum AnyEvent {
    RoomMessage(RoomMessageEventContent),
    Reaction(ReactionEventContent),
}

impl AnyEvent {
    async fn send(self, room: &mut matrix_sdk::room::Joined) -> anyhow::Result<()> {
        let _ = match self {
            AnyEvent::RoomMessage(e) => room.send(e, None).await?,
            AnyEvent::Reaction(e) => room.send(e, None).await?,
        };
        Ok(())
    }
}

async fn on_message(
    ev: SyncRoomMessageEvent,
    room: Room,
    client: Client,
    Ctx(ctx): Ctx<App>,
) -> anyhow::Result<()> {
    let mut room = if let Room::Joined(room) = room {
        room
    } else {
        // Ignore non-joined rooms events.
        return Ok(());
    };

    if ev.sender() == client.user_id().unwrap() {
        // Skip messages sent by the bot.
        return Ok(());
    }

    if let Some(unredacted) = ev.as_original() {
        let content = if let MessageType::Text(text) = &unredacted.content.msgtype {
            text.body.to_string()
        } else {
            // Ignore other kinds of messages at the moment.
            return Ok(());
        };

        tracing::trace!(
            "Received a message from {} in {}: {}",
            ev.sender(),
            room.room_id(),
            content,
        );

        // TODO ohnoes, locking across other awaits is bad
        // TODO Use a lock-free data-structure for the list of modules + put locks in the module
        // internal implementation?
        // TODO or create a new wasm instance per message \o/
        let ctx = ctx.inner.clone();
        let room_id = room.room_id().to_owned();

        let event_id = ev.event_id().to_owned();

        let new_actions = tokio::task::spawn_blocking(move || {
            let ctx = &mut *futures::executor::block_on(ctx.lock());

            let (store, modules) = ctx.modules.iter();

            if ev.sender() == ctx.admin_user_id {
                match try_handle_admin(
                    &content,
                    &ctx.admin_user_id,
                    &room_id,
                    store,
                    modules.clone(),
                    &mut ctx.room_resolver,
                ) {
                    None => {}
                    Some(actions) => {
                        tracing::trace!("handled by admin, skipping modules");
                        return actions;
                    }
                }
            }

            if let Some(actions) = try_handle_help(&content, ev.sender(), store, modules.clone()) {
                tracing::trace!("handled by help, skipping modules");
                return vec![actions];
            }

            for module in modules {
                tracing::trace!("trying to handle message with {}...", module.name());
                match module.handle(&mut *store, &content, ev.sender(), &room_id) {
                    Ok(actions) => {
                        if !actions.is_empty() {
                            // TODO support handling the same message with several handlers.
                            tracing::trace!("{} returned a response!", module.name());
                            return actions;
                        }
                    }
                    Err(err) => {
                        tracing::warn!("wasm module {} ran into an error: {err}", module.name());
                    }
                }
            }

            Vec::new()
        })
        .await?;

        let new_events = new_actions
            .into_iter()
            .map(|a| match a {
                wasm::Action::Respond(msg) => {
                    let content = if let Some(html) = msg.html {
                        RoomMessageEventContent::text_html(msg.text, html)
                    } else {
                        RoomMessageEventContent::text_plain(msg.text)
                    };
                    AnyEvent::RoomMessage(content)
                }
                wasm::Action::React(reaction) => {
                    let reaction =
                        ReactionEventContent::new(Relation::new(event_id.clone(), reaction));
                    AnyEvent::Reaction(reaction)
                }
            })
            .collect::<Vec<_>>();

        for event in new_events {
            event.send(&mut room).await?;
        }
    }

    Ok(())
}

/// Autojoin mixin.
async fn on_stripped_state_member(
    room_member: StrippedRoomMemberEvent,
    client: Client,
    room: Room,
) {
    if room_member.state_key != client.user_id().unwrap() {
        // the invite we've seen isn't for us, but for someone else. ignore
        return;
    }

    // looks like the room is an invited room, let's attempt to join then
    if let Room::Invited(room) = room {
        // The event handlers are called before the next sync begins, but
        // methods that change the state of a room (joining, leaving a room)
        // wait for the sync to return the new room state so we need to spawn
        // a new task for them.
        tokio::spawn(async move {
            tracing::debug!("Autojoining room {}", room.room_id());
            let mut delay = 1;

            while let Err(err) = room.accept_invitation().await {
                // retry autojoin due to synapse sending invites, before the
                // invited user can join for more information see
                // https://github.com/matrix-org/synapse/issues/4345
                tracing::warn!(
                    "Failed to join room {} ({err:?}), retrying in {delay}s",
                    room.room_id()
                );

                sleep(Duration::from_secs(delay)).await;
                delay *= 2;

                if delay > 3600 {
                    tracing::error!("Can't join room {} ({err:?})", room.room_id());
                    break;
                }
            }

            tracing::debug!("Successfully joined room {}", room.room_id());
        });
    }
}

/// Run the client for the given `BotConfig`.
pub async fn run(config: BotConfig) -> anyhow::Result<()> {
    let client = Client::builder()
        .server_name(config.home_server.as_str().try_into()?)
        .sled_store(&config.matrix_store_path, None)?
        .build()
        .await?;

    // Create the database, and try to find a device id.
    let db = Arc::new(unsafe { redb::Database::create(config.redb_path, 1024 * 1024)? });

    // First we need to log in.
    tracing::debug!("logging in...");
    let mut login_builder = client.login_username(&config.user_id, &config.password);

    let mut db_device_id = None;
    if let Some(device_id) =
        admin_table::read(&db, DEVICE_ID_ENTRY).context("reading device_id from the database")?
    {
        tracing::trace!("reusing previous device_id...");
        // the login builder keeps a reference to the previous device id string, so can't clone
        // db_device_id here, it has to outlive the login_builder.
        db_device_id = Some(String::from_utf8(device_id)?);
        login_builder = login_builder.device_id(db_device_id.as_ref().unwrap());
    }

    let resp = login_builder.send().await?;

    let resp_device_id = resp.device_id.to_string();
    if db_device_id.as_ref() != Some(&resp_device_id) {
        tracing::trace!("storign new device_id...");
        admin_table::write(&db, DEVICE_ID_ENTRY, resp_device_id.as_bytes())
            .context("writing new device_id into the database")?;
    }

    client
        .user_id()
        .context("impossible state: missing user id for the logged in bot?")?;

    // An initial sync to set up state and so our bot doesn't respond to old
    // messages. If the `StateStore` finds saved state in the location given the
    // initial sync will be skipped in favor of loading state from the store
    tracing::debug!("starting initial sync...");
    client.sync_once(SyncSettings::default()).await.unwrap();

    tracing::debug!("setting up app...");
    let client_copy = client.clone();
    let app_ctx = tokio::task::spawn_blocking(|| {
        AppCtx::new(client_copy, config.modules_paths, db, config.admin_user_id)
    })
    .await??;
    let app = App::new(app_ctx);

    let _watcher_guard = watcher(app.inner.clone()).await?;

    tracing::debug!("setup ready! now listening to incoming messages.");
    client.add_event_handler_context(app);
    client.add_event_handler(on_message);
    client.add_event_handler(on_stripped_state_member);

    // Note: this method will never return.
    let sync_settings = SyncSettings::default().token(client.sync_token().await.unwrap());

    tokio::select! {
        _ = handle_signals() => {
            // Exit :)
        }

        Err(err) = client.sync(sync_settings) => {
            anyhow::bail!(err);
        }
    }

    // Set bot presence to offline.
    let request = matrix_sdk::ruma::api::client::presence::set_presence::v3::Request::new(
        client.user_id().unwrap(),
        PresenceState::Offline,
    );

    client.send(request, None).await?;

    tracing::info!("properly exited, have a nice day!");
    Ok(())
}

async fn handle_signals() -> anyhow::Result<()> {
    use futures::StreamExt as _;
    use signal_hook::consts::signal::*;
    use signal_hook_tokio::*;

    let mut signals = Signals::new(&[SIGINT, SIGHUP, SIGQUIT, SIGTERM])?;
    let handle = signals.handle();

    while let Some(signal) = signals.next().await {
        match signal {
            SIGINT | SIGHUP | SIGQUIT | SIGTERM => {
                handle.close();
                break;
            }
            _ => {
                // Don't care.
            }
        }
    }

    Ok(())
}

async fn watcher(app: Arc<Mutex<AppCtx>>) -> anyhow::Result<Vec<notify::RecommendedWatcher>> {
    let modules_paths = { app.lock().await.modules_paths.clone() };

    let mut watchers = Vec::with_capacity(modules_paths.len());
    for modules_path in modules_paths {
        tracing::debug!(
            "setting up watcher on @ {}...",
            modules_path.to_string_lossy()
        );

        let rt_handle = tokio::runtime::Handle::current();
        let app = app.clone();
        let mut watcher = notify::recommended_watcher(
            move |res: Result<notify::Event, notify::Error>| match res {
                Ok(event) => {
                    // Only watch wasm files
                    if !event.paths.iter().any(|path| {
                        if let Some(ext) = path.extension() {
                            ext == "wasm"
                        } else {
                            false
                        }
                    }) {
                        return;
                    }

                    match event.kind {
                        notify::EventKind::Create(_)
                        | notify::EventKind::Modify(_)
                        | notify::EventKind::Remove(_) => {
                            // Trigger an update of the modules.
                            let app = app.clone();
                            rt_handle.spawn(async move {
                                AppCtx::set_needs_recompile(app).await;
                            });
                        }
                        notify::EventKind::Access(_)
                        | notify::EventKind::Any
                        | notify::EventKind::Other => {}
                    }
                }
                Err(e) => tracing::warn!("watch error: {e:?}"),
            },
        )?;

        watcher.watch(&modules_path, RecursiveMode::Recursive)?;
        watchers.push(watcher);
    }

    tracing::debug!("watcher setup done!");
    Ok(watchers)
}
