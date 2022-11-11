wit_bindgen_host_wasmtime_rust::generate!({
    import: "./wit/imports.wit",
    name: "imports"
});

wit_bindgen_host_wasmtime_rust::generate!({
    import: "./wit/log.wit",
    name: "logs"
});

wit_bindgen_host_wasmtime_rust::generate!({
    import: "./wit/sync-request.wit",
    name: "sync-request"
});

mod glue {
    wit_bindgen_host_wasmtime_rust::generate!({
        default: "./wit/exports.wit",
        name: "interface"
    });
}

use std::path::Path;

use sync_request::*;

use matrix_sdk::ruma::{RoomId, UserId};
use wasmtime::AsContextMut;

#[derive(Default)]
pub struct ModuleState {
    module_name: String,
    client: reqwest::blocking::Client,
}

impl imports::Imports for ModuleState {
    fn rand_u64(&mut self) -> anyhow::Result<u64> {
        Ok(rand::random())
    }
}

impl sync_request::SyncRequest for ModuleState {
    fn run_request(&mut self, req: Request) -> anyhow::Result<Result<Response, ()>> {
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
        let req = builder.build()?;

        let resp = self.client.execute(req)?;

        let status = match resp.status().as_u16() / 100 {
            2 => ResponseStatus::Success,
            _ => ResponseStatus::Error,
        };

        let body = resp.text().ok();

        Ok(Ok(Response { status, body }))
    }
}

impl log::Log for ModuleState {
    fn trace(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::trace!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn debug(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::debug!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn info(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::info!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn warn(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::warn!("{} - {msg}", self.module_name);
        Ok(())
    }
    fn error(&mut self, msg: String) -> anyhow::Result<()> {
        tracing::error!("{} - {msg}", self.module_name);
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct GuestState {
    imports: Vec<ModuleState>,
}

pub(crate) struct Module {
    name: String,
    exports: glue::Interface,
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
    ) -> anyhow::Result<Vec<glue::Message>> {
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
                module_name: name.clone(),
                client: reqwest::blocking::Client::default(),
            };

            let entry = store.data_mut().imports.len();
            store.data_mut().imports.push(module_state);

            let mut linker = wasmtime::component::Linker::<GuestState>::new(&engine);

            imports::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;
            log::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;
            sync_request::add_to_linker(&mut linker, move |s| &mut s.imports[entry])?;

            tracing::debug!(
                "compiling wasm module: {name} @ {}...",
                module_path.to_string_lossy()
            );

            let module = wasmtime::component::Component::from_file(&engine, &module_path)?;

            tracing::debug!("instantiating wasm module: {name}...");

            let (exports, instance) =
                glue::Interface::instantiate(&mut store, &module, &mut linker)?;

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
