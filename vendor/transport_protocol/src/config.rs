use std::collections::{HashMap, HashSet};

pub struct Config<Req, Res> {
    request_types: HashMap<String, (HashSet<String>, Box<Fn(Req) -> Res + Send + 'static>)>,
}

impl<Req, Res> Config<Req, Res> {
    pub fn new() -> Self {
        Self {
            request_types: HashMap::new(),
        }
    }

    pub fn on_request<RH: 'static>(
        mut self,
        request_type: &str,
        header_keys: &[&str],
        request_handler: RH,
    ) -> Self
    where
        RH: Fn(Req) -> Res + Send,
    {
        let header_keys = header_keys.into_iter().map(|key| (*key).into()).collect();
        let request_handler = Box::new(request_handler);

        self.request_types
            .insert(request_type.into(), (header_keys, request_handler));
        self
    }

    pub fn known_headers_for(&self, request_type: &str) -> Option<&HashSet<String>> {
        self.request_types.get(request_type).as_ref().map(|t| &t.0)
    }

    pub fn request_handler_for(&self, request_type: &str) -> Option<&Box<Fn(Req) -> Res + Send>> {
        self.request_types.get(request_type).as_ref().map(|t| &t.1)
    }
}
