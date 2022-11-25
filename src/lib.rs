mod wasm;

use anyhow::Context;
use matrix_sdk::{
    config::SyncSettings,
    event_handler::Ctx,
    room::Room,
    ruma::{
        events::room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, RoomMessageEventContent, SyncRoomMessageEvent},
        },
        OwnedUserId, UserId,
    },
    Client,
};
use notify::{RecursiveMode, Watcher};
use std::{env, path::PathBuf, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};
use wasm::{GuestState, Module, WasmModules};

/// The configuration to run a trininty instance with
/// FIXME: should be properly typed!
pub struct BotConfig {
    /// the matrix homeserver the bot should connect to
    pub home_server: String,
    /// the user_id to be used on the homeserver
    pub user_id: String,
    /// password to be used to log into the homeserver wth
    pub password: String,
    /// where to store the matrix-sdk internal data
    pub matrix_store_path: String,
    /// where to store the addtional database data
    pub redb_path: String,
    /// the admin user id for the bot
    pub admin_user_id: OwnedUserId,
}

/// Generate a `BotConfig` form the programm environment.
pub fn get_config_from_env() -> anyhow::Result<BotConfig> {
    // override environment variables with contents of .env file, unless they were already set
    // explicitly.
    dotenvy::dotenv().ok();

    let home_server = env::var("HOMESERVER").context("missing HOMESERVER variable")?;
    let user_id = env::var("BOT_USER_ID").context("missing bot user id in BOT_USER_ID")?;
    let password = env::var("BOT_PWD").context("missing bot user id in BOT_PWD")?;
    let matrix_store_path = env::var("MATRIX_STORE_PATH").context("missing MATRIX_STORE_PATH")?;
    let redb_path = env::var("REDB_PATH").context("missing REDB_PATH")?;

    let admin_user_id =
        env::var("ADMIN_USER_ID").context("missing admin user id in ADMIN_USER_ID")?;
    let admin_user_id = admin_user_id
        .try_into()
        .context("impossible to parse admin user id")?;

    Ok(BotConfig {
        home_server,
        user_id,
        password,
        matrix_store_path,
        admin_user_id,
        redb_path,
    })
}

pub(crate) type ShareableDatabase = Arc<redb::Database>;

struct AppCtx {
    client: Client,
    modules: WasmModules,
    modules_path: PathBuf,
    needs_recompile: bool,
    admin_user_id: OwnedUserId,
    db: ShareableDatabase,
}

impl AppCtx {
    /// Create a new `AppCtx`.
    ///
    /// Must be called from a blocking context.
    pub fn new(
        client: Client,
        modules_path: PathBuf,
        redb_path: String,
        admin_user_id: OwnedUserId,
    ) -> anyhow::Result<Self> {
        let db = Arc::new(unsafe { redb::Database::create(redb_path, 1024 * 1024)? });
        Ok(Self {
            client: client.clone(),
            modules: WasmModules::new(client, db.clone(), &modules_path)?,
            modules_path,
            needs_recompile: false,
            admin_user_id,
            db,
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

            match WasmModules::new(ptr.client.clone(), ptr.db.clone(), &ptr.modules_path) {
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

fn try_handle_admin<'a>(
    content: &str,
    sender: &UserId,
    store: &mut wasmtime::Store<GuestState>,
    modules: impl Clone + Iterator<Item = &'a Module>,
) -> Option<Vec<String>> {
    let Some(rest) = content.strip_prefix("!admin") else { return None };
    tracing::trace!("trying admin for {content}");
    if let Some(rest) = rest.strip_prefix(' ') {
        let rest = rest.trim();
        if let Some((module, rest)) = rest.split_once(" ").map(|(l, r)| (l, r.trim())) {
            let mut found = None;
            for m in modules {
                if m.name() == module {
                    found = match m.admin(&mut *store, rest.trim(), sender) {
                        Ok(msgs) => Some(msgs),
                        Err(err) => {
                            tracing::error!("error when handling admin command: {err:#}");
                            None
                        }
                    };
                    break;
                }
            }
            found.map(|messages| messages.into_iter().map(|msg| msg.content).collect())
        } else {
            Some(vec!["missing command".to_owned()])
        }
    } else {
        Some(vec!["missing module and command".to_owned()])
    }
}

fn try_handle_help<'a>(
    content: &str,
    store: &mut wasmtime::Store<GuestState>,
    modules: impl Clone + Iterator<Item = &'a Module>,
) -> Option<Vec<RoomMessageEventContent>> {
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
            .split_once(" ")
            .map(|(l, r)| (l, Some(r.trim())))
            .unwrap_or((rest, None));
        let mut found = None;
        for m in modules {
            if m.name() == module {
                found = m.help(&mut *store, topic.as_deref()).ok();
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

    Some(vec![RoomMessageEventContent::text_html(msg, html)])
}

async fn on_message(
    ev: SyncRoomMessageEvent,
    room: Room,
    client: Client,
    Ctx(ctx): Ctx<App>,
) -> anyhow::Result<()> {
    let room = if let Room::Joined(room) = room {
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

        let messages = tokio::task::spawn_blocking(move || {
            let ctx = &mut *futures::executor::block_on(ctx.lock());

            let mut outgoing_messages = Vec::new();

            let (store, modules) = ctx.modules.iter();

            if ev.sender() == &ctx.admin_user_id {
                if let Some(admin_messages) =
                    try_handle_admin(&content, &ctx.admin_user_id, store, modules.clone())
                {
                    tracing::trace!("handled by admin, skipping modules");
                    return admin_messages
                        .into_iter()
                        .map(|msg| RoomMessageEventContent::text_plain(msg))
                        .collect();
                }
            }

            if let Some(help_messages) = try_handle_help(&content, store, modules.clone()) {
                tracing::trace!("handled by help, skipping modules");
                return help_messages;
            }

            for module in modules {
                tracing::trace!("trying to handle message with {}...", module.name());

                match module.handle(&mut *store, &content, ev.sender(), &room_id) {
                    Ok(msgs) => {
                        let stop = !msgs.is_empty();

                        for msg in msgs {
                            let text = RoomMessageEventContent::text_plain(msg.content);
                            // TODO take msg.to into consideration, don't always answer the whole room
                            outgoing_messages.push(text);
                        }

                        // TODO support handling the same message with several handlers.
                        if stop {
                            tracing::trace!("{} returned a response!", module.name());
                            return outgoing_messages;
                        }
                    }

                    Err(err) => {
                        tracing::warn!("wasm module {} caused an error: {err}", module.name());
                    }
                }
            }

            outgoing_messages
        })
        .await?;

        for msg in messages {
            room.send(msg, None).await?;
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

pub async fn real_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::debug!("parsing config...");
    let config = get_config_from_env()?;

    tracing::debug!("creating client...");
    run(config).await
}

/// Run the client for the given `BotConfig`
pub async fn run(config: BotConfig) -> anyhow::Result<()> {
    let client = Client::builder()
        .server_name(config.home_server.as_str().try_into()?)
        .sled_store(&config.matrix_store_path, None)?
        .build()
        .await?;

    // First we need to log in.
    tracing::debug!("logging in...");
    client
        .login_username(&config.user_id, &config.password)
        .send()
        .await?;

    client
        .user_id()
        .context("missing user id for the logged in bot?")?;

    // An initial sync to set up state and so our bot doesn't respond to old
    // messages. If the `StateStore` finds saved state in the location given the
    // initial sync will be skipped in favor of loading state from the store
    tracing::debug!("starting initial sync...");
    client.sync_once(SyncSettings::default()).await.unwrap();

    tracing::debug!("setting up app...");
    let client_copy = client.clone();
    let app_ctx = tokio::task::spawn_blocking(|| {
        AppCtx::new(
            client_copy,
            "./modules/target/wasm32-unknown-unknown/release/".into(),
            config.redb_path,
            config.admin_user_id,
        )
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
    client.sync(sync_settings).await?;

    Ok(())
}

async fn watcher(app: Arc<Mutex<AppCtx>>) -> anyhow::Result<notify::RecommendedWatcher> {
    let modules_path = app.lock().await.modules_path.to_owned();
    tracing::debug!(
        "setting up watcher on @ {}...",
        modules_path.to_string_lossy()
    );

    let rt_handle = tokio::runtime::Handle::current();
    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| match res {
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
        })?;

    watcher.watch(&modules_path, RecursiveMode::Recursive)?;

    tracing::debug!("watcher setup done!");
    Ok(watcher)
}
