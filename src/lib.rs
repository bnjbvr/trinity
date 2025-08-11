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
            reaction::ReactionEventContent,
            relation::Annotation,
            room::{
                member::StrippedRoomMemberEvent,
                message::{MessageType, RoomMessageEventContent, SyncRoomMessageEvent},
                tombstone::OriginalSyncRoomTombstoneEvent,
            },
        },
        presence::PresenceState,
        OwnedUserId, RoomId, UserId,
    },
    Client, LoopCtrl, RoomState,
};
use notify::{RecursiveMode, Watcher};
use room_resolver::RoomResolver;
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};
use tracing::{debug, error, info, trace, warn};
use wasm::{Module, WasmModules};

use crate::admin_table::DEVICE_ID_ENTRY;

/// The configuration to run a trinity instance with.
#[derive(Deserialize)]
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
    /// module specific configuration to forward to corresponding handler.
    pub modules_config: Option<HashMap<String, HashMap<String, String>>>,
}

impl BotConfig {
    /// Generate a `BotConfig` from a TOML config file.
    ///
    /// If `path` matches `None`, will search for a file called `config.toml` in an XDG
    /// compliant configuration directory (e.g ~/.config/trinity/config.toml on Linux).
    pub fn from_config(path: Option<String>) -> anyhow::Result<Self> {
        let config_path = match path {
            Some(a) => a,
            None => {
                let dirs = directories::ProjectDirs::from("", "", "trinity")
                    .context("config file not found")?;
                let path = dirs.config_dir().join("config.toml");
                String::from(path.to_str().unwrap())
            }
        };
        let contents = fs::read_to_string(&config_path)?;
        let config: BotConfig = toml::from_str(&contents)?;

        debug!("Using configuration from {config_path}");
        Ok(config)
    }

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

        debug!("Using configuration from environment");
        Ok(Self {
            home_server,
            user_id,
            password,
            matrix_store_path,
            admin_user_id,
            redb_path,
            modules_paths,
            modules_config: None,
        })
    }
}

pub(crate) type ShareableDatabase = Arc<redb::Database>;

struct AppCtx {
    modules: WasmModules,
    modules_paths: Vec<PathBuf>,
    modules_config: HashMap<String, HashMap<String, String>>,
    engine: wasmtime::Engine,
    needs_recompile: bool,
    admin_user_id: OwnedUserId,
    db: ShareableDatabase,
    room_resolver: RoomResolver,
}

impl AppCtx {
    /// Create a new `AppCtx`.
    pub fn new(
        client: Client,
        modules_paths: Vec<PathBuf>,
        modules_config: HashMap<String, HashMap<String, String>>,
        db: ShareableDatabase,
        admin_user_id: OwnedUserId,
    ) -> anyhow::Result<Self> {
        let room_resolver = RoomResolver::new(client);

        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        config.cache_config_load_default()?;

        let engine = wasmtime::Engine::new(&config)?;

        Ok(Self {
            modules: WasmModules::new(&engine, db.clone(), &modules_paths, &modules_config)?,
            modules_paths,
            modules_config,
            needs_recompile: false,
            admin_user_id,
            db,
            room_resolver,
            engine,
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

        let mut ptr = futures::executor::block_on(async {
            tokio::time::sleep(Duration::new(1, 0)).await;
            ptr.lock().await
        });

        match WasmModules::new(
            &ptr.engine,
            ptr.db.clone(),
            &ptr.modules_paths,
            &ptr.modules_config,
        ) {
            Ok(modules) => {
                ptr.modules = modules;
                info!("successful hot reload!");
            }
            Err(err) => {
                error!("hot reload failed: {err:#}");
            }
        }

        ptr.needs_recompile = false;
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
    modules: impl Iterator<Item = &'a mut Module>,
    room_resolver: &mut RoomResolver,
) -> Option<Vec<wasm::Action>> {
    let Some(rest) = content.strip_prefix("!admin") else {
        return None;
    };

    trace!("trying admin for {content}");

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
                    found = match m.admin(rest.trim(), sender, target_room.as_str()) {
                        Ok(actions) => Some(actions),
                        Err(err) => {
                            error!("error when handling admin command: {err:#}");
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
    modules: impl Iterator<Item = &'a mut Module>,
) -> Option<wasm::Action> {
    let Some(rest) = content.strip_prefix("!help") else {
        return None;
    };

    // Special handling for help messages.
    let (msg, html) = if rest.trim().is_empty() {
        let mut msg = String::from("Available modules:");
        let mut html = String::from("Available modules: <ul>");
        for m in modules {
            let help = match m.help(None) {
                Ok(msg) => Some(msg),
                Err(err) => {
                    error!("error when handling help command: {err:#}");
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
                found = m.help(topic).ok();
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
    async fn send(self, room: &mut Room) -> anyhow::Result<()> {
        let _ = match self {
            AnyEvent::RoomMessage(e) => room.send(e).await?,
            AnyEvent::Reaction(e) => room.send(e).await?,
        };
        Ok(())
    }
}

/// After a room has been upgraded, automatically attempt to join the new room.
async fn on_room_upgrade(ev: OriginalSyncRoomTombstoneEvent, room: Room) {
    let content = ev.content;

    let alias_or_id = if let Some(alias) = room.canonical_alias() {
        alias.to_string()
    } else {
        room.room_id().to_string()
    };

    debug!(
        %alias_or_id,
        reason = content.body,
        "Room was upgraded, joining the new room.",
    );

    match room
        .client()
        .join_room_by_id(&content.replacement_room)
        .await
    {
        Ok(_) => {
            debug!("Successfully joined the upgraded room.");
        }
        Err(err) => {
            warn!(
                from = alias_or_id,
                to = %content.replacement_room,
                "Couldn't join the upgraded room: {err}"
            );
        }
    }
}

async fn on_message(
    ev: SyncRoomMessageEvent,
    mut room: Room,
    client: Client,
    Ctx(ctx): Ctx<App>,
) -> anyhow::Result<()> {
    if room.state() != RoomState::Joined {
        // Ignore non-joined rooms events.
        return Ok(());
    }

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

        trace!(
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

            if ev.sender() == ctx.admin_user_id {
                match try_handle_admin(
                    &content,
                    &ctx.admin_user_id,
                    &room_id,
                    ctx.modules.iter_mut(),
                    &mut ctx.room_resolver,
                ) {
                    None => {}
                    Some(actions) => {
                        trace!("handled by admin, skipping modules");
                        return actions;
                    }
                }
            }

            if let Some(actions) = try_handle_help(&content, ev.sender(), ctx.modules.iter_mut()) {
                trace!("handled by help, skipping modules");
                return vec![actions];
            }

            for module in ctx.modules.iter_mut() {
                trace!("trying to handle message with {}...", module.name());
                match module.handle(&content, ev.sender(), &room_id) {
                    Ok(actions) => {
                        if !actions.is_empty() {
                            // TODO support handling the same message with several handlers.
                            trace!("{} returned a response!", module.name());
                            return actions;
                        }
                    }
                    Err(err) => {
                        warn!("wasm module {} ran into an error: {err}", module.name());
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
                        ReactionEventContent::new(Annotation::new(event_id.clone(), reaction));
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

    // looks like the room is an invited room, let's attempt to join it, then.
    if room.state() == RoomState::Invited {
        // The event handlers are called before the next sync begins, but
        // methods that change the state of a room (joining, leaving a room)
        // wait for the sync to return the new room state so we need to spawn
        // a new task for them.
        tokio::spawn(async move {
            debug!("Autojoining room {}", room.room_id());
            let mut delay = 1;

            while let Err(err) = room.join().await {
                // retry autojoin due to synapse sending invites, before the
                // invited user can join for more information see
                // https://github.com/matrix-org/synapse/issues/4345
                warn!(
                    "Failed to join room {} ({err:?}), retrying in {delay}s",
                    room.room_id()
                );

                sleep(Duration::from_secs(delay)).await;
                delay *= 2;

                if delay > 3600 {
                    error!("Can't join room {} ({err:?})", room.room_id());
                    break;
                }
            }

            debug!("Successfully joined room {}", room.room_id());
        });
    }
}

async fn run_migrations(config: &BotConfig, db: &ShareableDatabase) -> anyhow::Result<()> {
    let version = admin_table::read_u64(&db, admin_table::VERSION_ENTRY)
        .context("reading version from the database")?
        .unwrap_or(0);

    if version < 1 {
        // If no version is set, we assume this is the first run, which happens after the sdk bump.
        // In this case, the state is likely outdated in the database, so we remove the database
        // directory first, and then drop the device id, so as to create a new one.
        debug!("running data migration 1: removing old database directory and dropping device id");

        // Remove the database directory.
        if let Err(err) = fs::remove_dir_all(&config.matrix_store_path) {
            warn!("failed to remove old database directory: {err:#}");
        }

        // Drop the device id, so that a new one is created on the next login.
        if let Err(err) =
            admin_table::remove(&db, DEVICE_ID_ENTRY).context("dropping device_id in the database")
        {
            warn!("failed to drop device_id: {err:#}");
        }

        // Bump the version to 1.
        admin_table::write_u64(&db, admin_table::VERSION_ENTRY, 1)
            .context("writing initial version into the database")?;
    }

    Ok(())
}

/// Run the client for the given `BotConfig`.
pub async fn run(config: BotConfig) -> anyhow::Result<()> {
    // Create the database, and try to find a device id.
    let db = Arc::new(unsafe { redb::Database::create(config.redb_path.clone(), 1024 * 1024)? });

    run_migrations(&config, &db).await?;

    let client = Client::builder()
        .server_name(config.home_server.as_str().try_into()?)
        .sqlite_store(&config.matrix_store_path, None)
        .build()
        .await?;

    // First we need to log in.
    debug!("logging in...");
    let mut login_builder = client
        .matrix_auth()
        .login_username(&config.user_id, &config.password);

    let mut db_device_id = None;
    if let Some(device_id) = admin_table::read_str(&db, DEVICE_ID_ENTRY)
        .context("reading device_id from the database")?
    {
        trace!("reusing previous device_id...");
        // the login builder keeps a reference to the previous device id string, so can't clone
        // db_device_id here, it has to outlive the login_builder.
        db_device_id = Some(device_id);
        login_builder = login_builder.device_id(db_device_id.as_ref().unwrap());
    }

    let resp = login_builder.send().await?;

    let resp_device_id = resp.device_id.to_string();
    if db_device_id.as_ref() != Some(&resp_device_id) {
        match db_device_id {
            Some(prev) => {
                warn!("overriding device_id (previous was {prev}, new is {resp_device_id})")
            }
            None => debug!("storing new device_id for the first time..."),
        }
        admin_table::write_str(&db, DEVICE_ID_ENTRY, &resp_device_id)
            .context("writing new device_id into the database")?;
    }

    let modules_config = config.modules_config.unwrap_or_else(HashMap::new);

    client
        .user_id()
        .context("impossible state: missing user id for the logged in bot?")?;

    // An initial sync to set up state and so our bot doesn't respond to old
    // messages. If the `StateStore` finds saved state in the location given the
    // initial sync will be skipped in favor of loading state from the store.
    debug!("starting initial sync...");
    client
        .sync_with_callback(SyncSettings::default(), |_| async { LoopCtrl::Break })
        .await
        .unwrap();

    debug!("setting up app...");
    let client_copy = client.clone();
    let app_ctx = AppCtx::new(
        client_copy,
        config.modules_paths,
        modules_config,
        db,
        config.admin_user_id,
    )?;
    let app = App::new(app_ctx);

    let _watcher_guard = watcher(app.inner.clone()).await?;

    debug!("setup ready! now listening to incoming messages.");
    client.add_event_handler_context(app);
    client.add_event_handler(on_message);
    client.add_event_handler(on_room_upgrade);
    client.add_event_handler(on_stripped_state_member);

    // Note: this method will never return.

    tokio::select! {
        _ = handle_signals() => {
            // Exit :)
        }

        Err(err) = client.sync(SyncSettings::default()) => {
            anyhow::bail!(err);
        }
    }

    // Set bot presence to offline.
    let request = matrix_sdk::ruma::api::client::presence::set_presence::v3::Request::new(
        client.user_id().unwrap().to_owned(),
        PresenceState::Offline,
    );

    client.send(request).await?;

    info!("properly exited, have a nice day!");
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
        debug!(
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
                Err(e) => warn!("watch error: {e:?}"),
            },
        )?;

        watcher.watch(&modules_path, RecursiveMode::Recursive)?;
        watchers.push(watcher);
    }

    debug!("watcher setup done!");
    Ok(watchers)
}
