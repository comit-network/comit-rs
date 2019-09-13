#[derive(Debug)]
pub enum Error<T> {
    FailedToStore(T),
    NotFound,
    Internal,
}

pub trait QueryRepository<T>: Send + Sync + 'static {
    fn all(&self) -> Box<dyn Iterator<Item = (String, T)>>;
    fn get(&self, id: String) -> Option<T>;
    fn save_with_id(&self, entity: T, id: String) -> Result<String, Error<T>>;
    fn delete(&self, id: String);
}
