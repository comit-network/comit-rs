use std::collections::{HashMap, HashSet};

pub struct Config<Req, Res> {
    known_headers: HashMap<String, HashSet<String>>,
    request_handlers: HashMap<String, Box<FnMut(Req) -> Res + Send + 'static>>,
}

impl<Req, Res> Config<Req, Res> {
    pub fn new() -> Self {
        Self {
            known_headers: HashMap::new(),
            request_handlers: HashMap::new(),
        }
    }

    pub fn on_request<RH: 'static>(
        mut self,
        request_type: &str,
        header_keys: &[&str],
        request_handler: RH,
    ) -> Self
    where
        RH: FnMut(Req) -> Res + Send,
    {
        let header_keys = header_keys.into_iter().map(|key| (*key).into()).collect();
        let request_handler = Box::new(request_handler);

        self.known_headers.insert(request_type.into(), header_keys);
        self.request_handlers
            .insert(request_type.into(), request_handler);

        self
    }

    pub fn known_headers_for(&self, request_type: &str) -> Option<&HashSet<String>> {
        self.known_headers.get(request_type)
    }

    pub fn request_handler_for(
        &mut self,
        request_type: &str,
    ) -> Option<&mut Box<FnMut(Req) -> Res + Send>> {
        self.request_handlers.get_mut(request_type)
    }
}
