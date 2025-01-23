use crate::wasm::apis::sync_request::trinity::api::sync_request;
use crate::wasm::GuestState;

wasmtime::component::bindgen!({
    path: "./wit/sync-request.wit",
    world: "sync-request-world"
});

use sync_request::*;

#[derive(Default)]
pub(super) struct SyncRequestApi {
    client: reqwest::blocking::Client,
}

impl SyncRequestApi {
    pub fn link(
        id: usize,
        linker: &mut wasmtime::component::Linker<GuestState>,
    ) -> anyhow::Result<()> {
        sync_request::add_to_linker(linker, move |s| &mut s.imports[id].apis.sync_request)
    }
}

impl sync_request::Host for SyncRequestApi {
    fn run_request(&mut self, req: Request) -> Result<Response, RunRequestError> {
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
            builder = builder.body(body);
        }
        let req = builder
            .build()
            .map_err(|err| RunRequestError::Builder(err.to_string()))?;

        let resp = self
            .client
            .execute(req)
            .map_err(|err| RunRequestError::Execute(err.to_string()))?;

        let status = match resp.status().as_u16() / 100 {
            2 => ResponseStatus::Success,
            _ => ResponseStatus::Error,
        };

        let body = resp.text().ok();

        Ok(Response { status, body })
    }
}
