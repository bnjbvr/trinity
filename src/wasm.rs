wit_bindgen_host_wasmtime_rust::export!("./wit/imports.wit");
wit_bindgen_host_wasmtime_rust::export!("./wit/log.wit");
wit_bindgen_host_wasmtime_rust::export!("./wit/sync-request.wit");

wit_bindgen_host_wasmtime_rust::import!("./wit/exports.wit");

use std::path::Path;

use imports::*;
use sync_request::*;

use matrix_sdk::ruma::{RoomId, UserId};
use wasmtime::AsContextMut;

#[derive(Default)]
pub struct ModuleState {
    module_name: String,
    client: reqwest::blocking::Client,
}

impl Imports for ModuleState {
    fn rand_u64(&mut self) -> u64 {
        rand::random()
    }
}

impl sync_request::SyncRequest for ModuleState {
    fn request(&mut self, req: Request<'_>) -> Result<Response, ()> {
        let url = req.url;
        let mut builder = match req.verb {
            RequestVerb::Get => self.client.get(url),
            RequestVerb::Put => self.client.put(url),
            RequestVerb::Delete => self.client.delete(url),
            RequestVerb::Post => self.client.post(url),
        };
        for header in req.headers {
            builder = builder.header(header.key, header.value);
        }
        if let Some(body) = req.body {
            builder = builder.body(body.to_owned());
        }
        let req = builder.build().map_err(|_| ())?;

        let resp = self.client.execute(req).map_err(|_| ())?;

        let status = match resp.status().as_u16() / 100 {
            2 => ResponseStatus::Success,
            _ => ResponseStatus::Error,
        };

        let body = resp.text().ok();

        Ok(Response { status, body })
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
    /// Create a new collection of wasm modules.
    ///
    /// Must be called from a blocking context.
    pub fn new(modules_path: &Path) -> anyhow::Result<Self> {
        tracing::debug!("setting up wasm context...");

        let engine = wasmtime::Engine::default();

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
                module_name: name.clone(),
                client: reqwest::blocking::Client::default(),
            };

            let entry = store.data_mut().imports.len();
            store.data_mut().imports.push(module_state);

            let mut linker = wasmtime::Linker::<GuestState>::new(&engine);

            imports::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;
            log::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;
            sync_request::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;

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
