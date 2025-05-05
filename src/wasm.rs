mod module {
    wasmtime::component::bindgen!({
        path: "./wit/trinity-module.wit",
    });
}

use crate::wasm::module::exports::trinity::module::messaging;
pub(crate) use messaging::Action;
pub(crate) use messaging::Message;
use module::TrinityModule;
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use wasmtime::Store;

mod apis;

use std::collections::HashMap;
use std::path::PathBuf;

use matrix_sdk::ruma::{RoomId, UserId};

use crate::{wasm::apis::Apis, ShareableDatabase};

pub struct ModuleState {
    apis: Apis,
}

pub(crate) struct Module {
    name: String,
    instance: TrinityModule,
    pub store: Store<ModuleState>,
}

impl Module {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn help(&mut self, topic: Option<&str>) -> anyhow::Result<String> {
        self.instance
            .trinity_module_messaging()
            .call_help(&mut self.store, topic)
    }

    pub fn admin(
        &mut self,
        cmd: &str,
        sender: &UserId,
        room: &str,
    ) -> anyhow::Result<Vec<messaging::Action>> {
        self.instance.trinity_module_messaging().call_admin(
            &mut self.store,
            cmd,
            sender.as_str(),
            room,
        )
    }

    pub fn handle(
        &mut self,
        content: &str,
        sender: &UserId,
        room: &RoomId,
    ) -> anyhow::Result<Vec<messaging::Action>> {
        self.instance.trinity_module_messaging().call_on_msg(
            &mut self.store,
            content,
            sender.as_str(),
            "author name NYI",
            room.as_str(),
        )
    }
}

#[derive(Default)]
pub(crate) struct WasmModules {
    modules: Vec<Module>,
}

impl WasmModules {
    /// Create a new collection of wasm modules.
    pub fn new(
        engine: &wasmtime::Engine,
        db: ShareableDatabase,
        modules_paths: &[PathBuf],
        modules_config: &HashMap<String, HashMap<String, String>>,
    ) -> anyhow::Result<Self> {
        tracing::debug!("setting up wasm context...");

        let mut compiled_modules = Vec::new();

        tracing::debug!("precompiling wasm modules...");
        for modules_path in modules_paths {
            tracing::debug!(
                "looking for modules in {}...",
                modules_path.to_string_lossy()
            );

            // Collect all the modules paths and names.
            let mut path_and_names = vec![];
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

                path_and_names.push((module_path, name));
            }

            // Compile and re-init all the modules in parallel.
            let batch: Vec<_> = path_and_names
                .into_par_iter()
                .map(|(module_path, name)| -> anyhow::Result<Module> {
                    let span = tracing::debug_span!("compiling module", name = %name, );
                    let _scope = span.enter();

                    tracing::debug!(
                        path = module_path.to_str().unwrap_or("<invalid path>"),
                        "initializing: creating APIs"
                    );
                    let module_state = ModuleState {
                        apis: Apis::new(name.clone(), db.clone())?,
                    };

                    let mut store = wasmtime::Store::new(&engine, module_state);
                    let mut linker = wasmtime::component::Linker::new(&engine);

                    apis::Apis::link(&mut linker)?;

                    tracing::debug!("compiling");
                    let component =
                        wasmtime::component::Component::from_file(&engine, &module_path)?;

                    tracing::debug!("instantiating");
                    let instance =
                        module::TrinityModule::instantiate(&mut store, &component, &linker)?;

                    // Convert the module config to Vec of tuples to satisfy wasm interface types.
                    let init_config: Option<Vec<(String, String)>> = modules_config
                        .get(&name)
                        .map(|mc| Vec::from_iter(mc.clone()));

                    tracing::debug!("calling module's init() function");
                    instance
                        .trinity_module_messaging()
                        .call_init(&mut store, init_config.as_deref())?;

                    tracing::debug!("great success!");
                    Ok(Module {
                        name,
                        instance,
                        store,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            compiled_modules.extend(batch);
        }

        Ok(Self {
            modules: compiled_modules,
        })
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Module> {
        self.modules.iter_mut()
    }
}
