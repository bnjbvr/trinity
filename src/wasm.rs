mod module {
    wasmtime::component::bindgen!({
        path: "./wit/trinity-module.wit",
    });
}

use crate::wasm::module::exports::trinity::module::messaging;
pub(crate) use messaging::Action;
pub(crate) use messaging::Message;
use module::TrinityModule;

mod apis;

use std::collections::HashMap;
use rayon::prelude::*;
use std::path::PathBuf;

use matrix_sdk::ruma::{RoomId, UserId};
use wasmtime::AsContextMut;

use crate::{wasm::apis::Apis, ShareableDatabase};

pub(crate) struct GuestState {
    apis: Apis,
}

impl Default for GuestState {
    fn default() -> Self {
        panic!("GuestState requires Apis to be initialized and cannot be created with default()")
    }
}

pub(crate) struct Module {
    name: String,
    instance: TrinityModule,
}

impl Module {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn help(
        &self,
        mut store: impl AsContextMut<Data = GuestState>,
        topic: Option<&str>,
    ) -> anyhow::Result<String> {
        self.instance
            .trinity_module_messaging()
            .call_help(&mut store, topic)
    }

    pub fn admin(
        &self,
        mut store: impl AsContextMut<Data = GuestState>,
        cmd: &str,
        sender: &UserId,
        room: &str,
    ) -> anyhow::Result<Vec<messaging::Action>> {
        self.instance
            .trinity_module_messaging()
            .call_admin(&mut store, cmd, sender.as_str(), room)
    }

    pub fn handle(
        &self,
        mut store: impl AsContextMut<Data = GuestState>,
        content: &str,
        sender: &UserId,
        room: &RoomId,
    ) -> anyhow::Result<Vec<messaging::Action>> {
        self.instance.trinity_module_messaging().call_on_msg(
            &mut store,
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
    modules: Vec<(Module, WasmStore)>,
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

        tracing::debug!("precompiling wasm modules...");
        // First, collect all module names and paths in a non-parallel loop
        let module_entries: Vec<(String, PathBuf)> = modules_paths
            .iter()
            .flat_map(|modules_path| {
                tracing::debug!(
                    "looking for modules in {}...",
                    modules_path.to_string_lossy()
                );
                std::fs::read_dir(modules_path)
                    .into_iter()
                    .flat_map(|entries| entries.filter_map(Result::ok))
                    .filter_map(|entry| {
                        let module_path = entry.path();
                        if module_path.extension().map_or(true, |ext| ext != "wasm") {
                            return None;
                        }

                        let name = module_path
                            .file_stem()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_else(|| module_path.to_string_lossy())
                            .to_string();

                        Some((name, module_path))
                    })
            })
            .collect();

        // Then, compile all modules in parallel using rayon and pair each Module with its own Store
        let compiled_modules: Vec<(Module, WasmStore)> = module_entries
            .into_par_iter() // Use parallel iterator for compilation
            .filter_map(|(name, module_path)| {
                tracing::debug!("creating APIs...");
                // Create APIs directly for the module
                let apis = Apis::new(name.clone(), db.clone()).ok()?;

                // Create a thread-local store for each module
                let mut thread_store = wasmtime::Store::new(&engine, GuestState { apis });

                let mut linker = wasmtime::component::Linker::<GuestState>::new(&engine);

                if apis::Apis::link(&mut linker).is_err() {
                    return None;
                }

                tracing::debug!(
                    "compiling wasm module: {name} @ {}...",
                    module_path.to_string_lossy()
                );

                let component = wasmtime::component::Component::from_file(&engine, &module_path).ok()?;

                tracing::debug!("instantiating wasm component: {name}...");

                let instance = module::TrinityModule::instantiate(&mut thread_store, &component, &linker).ok()?;

                let init_config: Option<Vec<(String, String)>> = modules_config
                    .get(&name)
                    .map(|mc| Vec::from_iter(mc.clone()));

                tracing::debug!("calling module's init function...");
                if instance
                    .trinity_module_messaging()
                    .call_init(&mut thread_store, init_config.as_deref())
                    .is_err()
                {
                    return None;
                }

                tracing::debug!("great success!");
                // Return both the Module and its associated Store
                Some((Module { name, instance }, thread_store))
            })
            .collect();

        Ok(Self {
            modules: compiled_modules,
        })
    }

    pub fn handle_message(
        &mut self,
        content: &str,
        sender: &UserId,
        room: &RoomId,
    ) -> Vec<(String, Vec<messaging::Action>)> {
        self.modules
            .iter_mut()
            .filter_map(|(module, store)| {
                module
                    .handle(store, content, sender, room)
                    .ok()
                    .filter(|actions| !actions.is_empty())
                    .map(|actions| (module.name().to_owned(), actions))
            })
            .collect()
    }

    pub fn help(&mut self, topic: Option<&str>) -> Vec<(&str, anyhow::Result<String>)> {
        self.modules
            .iter_mut()
            .map(|(module, store)| {
                (module.name(), module.help(store, topic))
            })
            .collect()
    }

    pub fn help_for(&mut self, name: &str, topic: Option<&str>) -> Option<anyhow::Result<String>> {
        self.modules
            .iter_mut()
            .find(|(module, _)| module.name() == name)
            .map(|(module, store)| module.help(store, topic))
    }
    
    pub(crate) fn iter(&mut self) -> impl Iterator<Item = (&Module, &mut WasmStore)> + '_ {
        self.modules.iter_mut().map(|(module, store)| (&*module, store))
    }

    pub fn refresh(
        &mut self,
        old_name: &str,
        db: ShareableDatabase,
        component_path: &PathBuf,
        config: Option<&HashMap<String, String>>,
    ) -> anyhow::Result<()> {
        // Find the module to refresh
        if let Some(idx) = self.modules.iter().position(|(m, _)| m.name() == old_name) {
            // Create a new config and engine
            let mut wasmtime_config = wasmtime::Config::new();
            wasmtime_config.wasm_component_model(true);
            let engine = wasmtime::Engine::new(&wasmtime_config)?;

            // Extract the name for logs and creating APIs
            let name = component_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| component_path.to_string_lossy())
                .to_string();

            tracing::debug!("creating APIs for refreshed module...");
            let apis = Apis::new(name.clone(), db)?;

            // Create a new store for this module with direct API access
            let mut store = wasmtime::Store::new(&engine, GuestState { apis });

            let mut linker = wasmtime::component::Linker::<GuestState>::new(&engine);

            apis::Apis::link(&mut linker)?;

            tracing::debug!(
                "compiling refreshed wasm module: {name} @ {}...",
                component_path.to_string_lossy()
            );

            let component = wasmtime::component::Component::from_file(&engine, component_path)?;

            tracing::debug!("instantiating refreshed wasm component: {name}...");

            let instance = module::TrinityModule::instantiate(&mut store, &component, &linker)?;

            let init_config = config.map(|c| Vec::from_iter(c.clone()));

            tracing::debug!("calling refreshed module's init function...");
            instance
                .trinity_module_messaging()
                .call_init(&mut store, init_config.as_deref())?;

            // Replace the old module with the new one
            self.modules[idx] = (Module { name, instance }, store);
            Ok(())
        } else {
            anyhow::bail!("module {} not found", old_name);
        }
    }
}
