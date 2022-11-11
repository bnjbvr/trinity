mod module {
    wit_bindgen_host_wasmtime_rust::generate!({
        default: "./wit/trinity-module.wit",
        name: "interface"
    });
}

mod apis;

use std::path::Path;

use matrix_sdk::ruma::{RoomId, UserId};
use wasmtime::AsContextMut;

use crate::wasm::apis::Apis;

pub struct ModuleState {
    apis: Apis,
}

#[derive(Default)]
pub(crate) struct GuestState {
    imports: Vec<ModuleState>,
}

pub(crate) struct Module {
    name: String,
    exports: module::Interface,
    _instance: wasmtime::component::Instance,
}

impl Module {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn handle(
        &self,
        store: impl AsContextMut<Data = GuestState>,
        content: &str,
        sender: &UserId,
        room: &RoomId,
    ) -> anyhow::Result<Vec<module::Message>> {
        let msgs = self.exports.on_msg(
            store,
            content,
            sender.as_str(),
            "author name NYI",
            room.as_str(),
        )?;
        Ok(msgs)
    }
}

pub(crate) type WasmStore = wasmtime::Store<GuestState>;

pub(crate) struct WasmModules {
    store: WasmStore,
    modules: Vec<Module>,
}

impl WasmModules {
    /// Create a new collection of wasm modules.
    ///
    /// Must be called from a blocking context.
    pub fn new(modules_path: &Path) -> anyhow::Result<Self> {
        tracing::debug!("setting up wasm context...");

        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);

        let engine = wasmtime::Engine::new(&config)?;

        let mut compiled_modules = Vec::new();

        let state = GuestState::default();

        let mut store = wasmtime::Store::new(&engine, state);

        tracing::debug!("precompiling wasm modules...");
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

            let module_state = ModuleState {
                apis: Apis::new(name.clone()),
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
                module::Interface::instantiate(&mut store, &component, &mut linker)?;

            tracing::debug!("calling module's init function...");
            exports.init(&mut store)?;

            tracing::debug!("great success!");
            compiled_modules.push(Module {
                name,
                exports,
                _instance: instance,
            });
        }

        Ok(Self {
            store,
            modules: compiled_modules,
        })
    }

    pub(crate) fn iter(&mut self) -> (&mut WasmStore, impl Iterator<Item = &Module>) {
        (&mut self.store, self.modules.iter())
    }
}
