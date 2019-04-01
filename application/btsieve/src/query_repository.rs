#[derive(Debug)]
pub enum Error<T> {
	FailedToStore(T),
	NotFound,
	Internal,
}

pub trait QueryRepository<T>: Send + Sync + 'static {
	fn all(&self) -> Box<dyn Iterator<Item = (u32, T)>>;
	fn get(&self, id: u32) -> Option<T>;
	fn save(&self, entity: T) -> Result<u32, Error<T>>;
	fn delete(&self, id: u32);
}
