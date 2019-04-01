use futures::Future;
use std::collections::{HashMap, HashSet};

pub type ResponseFuture<I> = dyn Future<Item = I, Error = ()> + Send + 'static;
pub type RequestFn<Req, Res> = dyn FnMut(Req) -> Box<ResponseFuture<Res>> + Send + 'static;

#[derive(DebugStub)]
pub struct Config<Req, Res> {
	known_headers: HashMap<String, HashSet<String>>,
	#[debug_stub = "RequestHandlers"]
	request_handlers: HashMap<String, Box<RequestFn<Req, Res>>>,
}

impl<Req, Res> Default for Config<Req, Res> {
	fn default() -> Self {
		Self {
			known_headers: HashMap::new(),
			request_handlers: HashMap::new(),
		}
	}
}

impl<Req, Res> Config<Req, Res> {
	pub fn on_request<RH: 'static>(
		mut self,
		request_type: &str,
		header_keys: &[&str],
		request_handler: RH,
	) -> Self
	where
		RH: FnMut(Req) -> Box<ResponseFuture<Res>> + Send + 'static,
	{
		let header_keys = header_keys.iter().map(|key| (*key).into()).collect();

		let request_handler = Box::new(request_handler);

		self.known_headers.insert(request_type.into(), header_keys);
		self.request_handlers
			.insert(request_type.into(), request_handler);

		self
	}

	pub fn known_headers_for(&self, request_type: &str) -> Option<&HashSet<String>> {
		self.known_headers.get(request_type)
	}

	#[allow(clippy::borrowed_box)]
	pub fn request_handler_for(
		&mut self,
		request_type: &str,
	) -> Option<&mut Box<(RequestFn<Req, Res>)>> {
		self.request_handlers.get_mut(request_type)
	}
}
