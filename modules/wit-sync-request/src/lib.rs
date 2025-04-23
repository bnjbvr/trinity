mod sync_request_world;
use sync_request_world::trinity::api::sync_request as wit;

use std::collections::HashMap;

pub use wit::ResponseStatus;

/// A log implementation based on calls to the host.
pub struct Request {
    verb: wit::RequestVerb,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl Request {
    pub fn get(url: &str) -> Self {
        Self {
            verb: wit::RequestVerb::Get,
            url: url.to_owned(),
            headers: Default::default(),
            body: None,
        }
    }

    pub fn put(url: &str) -> Self {
        Self {
            verb: wit::RequestVerb::Put,
            url: url.to_owned(),
            headers: Default::default(),
            body: None,
        }
    }

    pub fn delete(url: &str) -> Self {
        Self {
            verb: wit::RequestVerb::Delete,
            url: url.to_owned(),
            headers: Default::default(),
            body: None,
        }
    }

    pub fn post(url: &str) -> Self {
        Self {
            verb: wit::RequestVerb::Post,
            url: url.to_owned(),
            headers: Default::default(),
            body: None,
        }
    }

    pub fn header(mut self, key: &str, val: &str) -> Self {
        let prev = self.headers.insert(key.to_owned(), val.to_owned());
        if prev.is_some() {
            log::warn!("overriding header {}", key);
        }
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        if self.body.is_some() {
            log::warn!("overriding request body");
        }
        self.body = Some(body.to_owned());
        self
    }

    pub fn run(self) -> Result<wit::Response, wit::RunRequestError> {
        let headers: Vec<_> = self
            .headers
            .into_iter()
            .map(|(key, value)| wit::RequestHeader { key, value })
            .collect();
        let req = wit::Request {
            verb: self.verb,
            url: self.url,
            headers,
            body: self.body,
        };
        wit::run_request(&req)
    }
}
