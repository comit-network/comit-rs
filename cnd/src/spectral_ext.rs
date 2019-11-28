use spectral::{result::ResultAssertions, AssertionFailure, Spec};
use std::fmt::{Debug, Display};

pub trait AnyhowResultAssertions<'s, T> {
    fn is_inner_err<E>(&'s mut self) -> Spec<'s, E>
    where
        T: Debug,
        E: Display + Debug + Send + Sync + 'static;
}

impl<'s, T> AnyhowResultAssertions<'s, T> for Spec<'s, anyhow::Result<T>> {
    fn is_inner_err<E>(&'s mut self) -> Spec<'s, E>
    where
        T: Debug,
        E: Display + Debug + Send + Sync + 'static,
    {
        let e = self.is_err().subject;

        match e.downcast_ref() {
            Some(inner) => Spec {
                subject: inner,
                subject_name: self.subject_name,
                location: self.location.clone(),
                description: self.description,
            },
            None => {
                AssertionFailure::from_spec(self)
                    .with_expected("error that can be downcasted".to_string())
                    .with_actual("error could not be downcasted".to_string())
                    .fail();

                unreachable!();
            }
        }
    }
}
