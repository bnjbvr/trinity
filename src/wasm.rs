mod module {
    wasmtime::component::bindgen!({
        path: "./wit/trinity-module.wit",
    });
}

use crate::wasm::module::exports::trinity::module::messaging;
pub(crate) use messaging::Action;
pub(crate) use messaging::Message;

mod apis;

use std::collections::HashMap;
use std::path::PathBuf;

use matrix_sdk::ruma::{RoomId, UserId};
use wasmtime::AsContextMut;

use crate::{wasm::apis::Apis, ShareableDatabase};

pub struct ModuleState {
    apis: Apis,
}

#[derive(Default)]
pub(crate) struct GuestState {
    imports: Vec<ModuleState>,
}

pub(crate) struct Module {
    name: String,
    exports: module::TrinityModule,
    _instance: wasmtime::component::Instance,
}

impl Module {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn help(
        &self,
        store: impl AsContextMut<Data = GuestState>,
        topic: Option<&str>,
    ) -> anyhow::Result<String> {
        self.exports
            .trinity_module_messaging()
            .call_help(store, topic)
    }

    pub fn admin(
        &self,
        store: impl AsContextMut<Data = GuestState>,
        cmd: &str,
        sender: &UserId,
        room: &str,
    ) -> anyhow::Result<Vec<messaging::Action>> {
        self.exports
            .trinity_module_messaging()
            .call_admin(store, cmd, sender.as_str(), room)
    }

    pub fn handle(
        &self,
        store: impl AsContextMut<Data = GuestState>,
        content: &str,
        sender: &UserId,
        room: &RoomId,
    ) -> anyhow::Result<Vec<messaging::Action>> {
        self.exports.trinity_module_messaging().call_on_msg(
            store,
            content,
            sender.as_str(),
            "author name NYI",
            room.as_str(),
        )
    }
}

pub(crate) type WasmStore = wasmtime::Store<GuestState>;

#[derive(Default)]
pub(crate) struct WasmModules {
    store: WasmStore,
    modules: Vec<Module>,
}

impl WasmModules {
    /// Create a new collection of wasm modules.
    ///
    /// Must be called from a blocking context.
    pub fn new(
        db: ShareableDatabase,
        modules_paths: &[PathBuf],
        modules_config: &HashMap<String, HashMap<String, String>>,
    ) -> anyhow::Result<Self> {
        tracing::debug!("setting up wasm context...");

        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);

        let engine = wasmtime::Engine::new(&config)?;

        let mut compiled_modules = Vec::new();

        let state = GuestState::default();

        let mut store = wasmtime::Store::new(&engine, state);

        tracing::debug!("precompiling wasm modules...");
        for modules_path in modules_paths {
            tracing::debug!(
                "looking for modules in {}...",
                modules_path.to_string_lossy()
            );
            for module_path in std::fs::read_dir(modules_path)? {
                let module_path = module_path?.path();

                if module_path.extension().map_or(true, |ext| ext != "wasm") {
                    continue;
                }

                let name = module_path
                    .file_stem()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_else(|| module_path.to_string_lossy())
                    .to_string();

                tracing::debug!("creating APIs...");
                let module_state = ModuleState {
                    apis: Apis::new(name.clone(), db.clone())?,
                };

                let entry = store.data_mut().imports.len();
                store.data_mut().imports.push(module_state);

                let mut linker = wasmtime::component::Linker::<GuestState>::new(&engine);

                apis::Apis::link(entry, &mut linker)?;

                tracing::debug!(
                    "compiling wasm module: {name} @ {}...",
                    module_path.to_string_lossy()
                );

                let component = wasmtime::component::Component::from_file(&engine, &module_path)?;

                tracing::debug!("instantiating wasm component: {name}...");

                let (exports, instance) =
                    module::TrinityModule::instantiate(&mut store, &component, &linker)?;

                // Convert the module config to Vec of tuples to satisfy wasm interface types.
                let init_config: Option<Vec<(String, String)>> = modules_config
                    .get(&name)
                    .map(|mc| Vec::from_iter(mc.clone()));

                tracing::debug!("calling module's init function...");
                exports
                    .trinity_module_messaging()
                    .call_init(&mut store, init_config.as_deref())?;

                tracing::debug!("great success!");
                compiled_modules.push(Module {
                    name,
                    exports,
                    _instance: instance,
                });
            }
        }

        Ok(Self {
            store,
            modules: compiled_modules,
        })
    }

    pub(crate) fn iter(&mut self) -> (&mut WasmStore, impl Clone + Iterator<Item = &Module>) {
        (&mut self.store, self.modules.iter())
    }
}
