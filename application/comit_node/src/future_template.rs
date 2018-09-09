use tokio::prelude::*;

pub trait FutureTemplate<D> {
    type Future: Future + Sized;

    fn into_future(self, dependencies: D) -> Self::Future;
}

pub struct FutureFactory<D> {
    dependencies: D,
}

impl<D: Clone> FutureFactory<D> {
    pub fn new(dependencies: D) -> Self {
        FutureFactory { dependencies }
    }

    pub fn create_future_from_template<T: FutureTemplate<D>>(
        &self,
        future_template: T,
    ) -> <T as FutureTemplate<D>>::Future {
        future_template.into_future(self.dependencies.clone())
    }
}
