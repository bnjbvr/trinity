wit_bindgen_host_wasmtime_rust::export!("./wit/imports.wit");
wit_bindgen_host_wasmtime_rust::export!("./wit/log.wit");
wit_bindgen_host_wasmtime_rust::import!("./wit/exports.wit");

use std::path::Path;

use imports::*;
use matrix_sdk::ruma::{RoomId, UserId};
use wasmtime::AsContextMut;

#[derive(Default)]
pub struct ModuleState {
    module_name: String,
}

impl Imports for ModuleState {
    fn rand_u64(&mut self) -> u64 {
        rand::random()
    }
}

impl log::Log for ModuleState {
    fn trace(&mut self, msg: &str) {
        tracing::trace!("{} - {msg}", self.module_name);
    }
    fn debug(&mut self, msg: &str) {
        tracing::debug!("{} - {msg}", self.module_name);
    }
    fn info(&mut self, msg: &str) {
        tracing::info!("{} - {msg}", self.module_name);
    }
    fn warn(&mut self, msg: &str) {
        tracing::warn!("{} - {msg}", self.module_name);
    }
    fn error(&mut self, msg: &str) {
        tracing::error!("{} - {msg}", self.module_name);
    }
}

#[derive(Default)]
pub(crate) struct GuestState {
    imports: Vec<ModuleState>,
    exports: exports::ExportsData,
}

pub(crate) struct Module {
    name: String,
    exports: exports::Exports<GuestState>,
    _instance: wasmtime::Instance,
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
    ) -> anyhow::Result<Vec<exports::Message>> {
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
    pub fn new(modules_path: &Path) -> anyhow::Result<Self> {
        tracing::debug!("setting up wasm context...");

        let engine = wasmtime::Engine::default();

        let mut compiled_modules = Vec::new();

        let state = GuestState::default();

        // A `Store` is what will own instances, functions, globals, etc. All wasm
        // items are stored within a `Store`, and it's what we'll always be using to
        // interact with the wasm world. Custom data can be stored in stores but for
        // now we just use `()`.
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
                module_name: name.clone(),
            };

            let entry = store.data_mut().imports.len();
            store.data_mut().imports.push(module_state);

            let mut linker = wasmtime::Linker::<GuestState>::new(&engine);

            imports::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;
            log::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;

            tracing::debug!(
                "compiling wasm module: {name} @ {}...",
                module_path.to_string_lossy()
            );
            let module = wasmtime::Module::from_file(&engine, &module_path)?;

            tracing::debug!("instantiating wasm module: {name}...");
            let (exports, instance) =
                exports::Exports::instantiate(&mut store, &module, &mut linker, |s| {
                    &mut s.exports
                })?;

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
