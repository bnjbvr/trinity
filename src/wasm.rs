mod module {
    wasmtime::component::bindgen!({
        path: "./wit/trinity-module.wit",
    });
}

use crate::wasm::module::exports::trinity::module::messaging;
pub(crate) use messaging::Action;
pub(crate) use messaging::Message;
use module::TrinityModule;

pub(crate) mod apis;

use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

use matrix_sdk::ruma::{RoomId, UserId};
use wasmtime::AsContextMut;

use crate::{wasm::apis::Apis, ShareableDatabase};

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
        mut store: impl AsContextMut<Data = Apis>,
        topic: Option<&str>,
    ) -> anyhow::Result<String> {
        self.instance
            .trinity_module_messaging()
            .call_help(&mut store, topic)
    }

    pub fn admin(
        &self,
        mut store: impl AsContextMut<Data = Apis>,
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
        mut store: impl AsContextMut<Data = Apis>,
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

pub(crate) type WasmStore = wasmtime::Store<Apis>;

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
                let apis = match Apis::new(name.clone(), db.clone()) {
                    Ok(apis) => apis,
                    Err(_) => return None,
                };

                // Create a thread-local store for each module
                let mut thread_store = wasmtime::Store::new(&engine, apis);

                let mut linker = wasmtime::component::Linker::<Apis>::new(&engine);

                if apis::Apis::link(&mut linker).is_err() {
                    return None;
                }

                tracing::debug!(
                    "compiling wasm module: {name} @ {}...",
                    module_path.to_string_lossy()
                );

                let component =
                    wasmtime::component::Component::from_file(&engine, &module_path).ok()?;

                tracing::debug!("instantiating wasm component: {name}...");

                let instance =
                    module::TrinityModule::instantiate(&mut thread_store, &component, &linker)
                        .ok()?;

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
    
    pub(crate) fn iter(&mut self) -> impl Iterator<Item = (&Module, &mut WasmStore)> + '_ {
        self.modules
            .iter_mut()
            .map(|(module, store)| (&*module, store))
    }
}
