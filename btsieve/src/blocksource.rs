use futures::Stream;

#[derive(Debug)]
pub enum Error<S> {
    /// Some `BlockSource` implementations use polling to continuously check for
    /// new blocks.
    ///
    /// This error variant represents a failure on the timer level.
    Timer(tokio::timer::Error),

    /// Represents errors from the underlying source of a `BlockSource`.
    Source(S),
}

/// Abstracts over a source for retrieving blocks
pub trait BlockSource {
    type Block;
    type Error;

    /// Returns a continuous stream of new blocks
    ///
    /// These blocks **do not** necessarily form a blockchain, i.e. they MAY
    /// not be connected. This stream just returns blocks as the underlying
    /// source learns about them. It is also not guaranteed that the blocks
    /// returned are unique, i.e. the same block may be returned several times.
    ///
    /// A Stream by itself does not terminate if it emits an error. However, if
    /// you use a combinator like `for_each` your stream will terminate upon the
    /// first error. Implementations of `BlockSource` are strongly
    /// encouraged to only bubble up errors that are not recoverable.
    ///
    /// For example, an implementation that polls an HTTP endpoint might not
    /// want to bubble up timeout errors because they might only be due to
    /// temporary network outages.
    fn blocks(&self) -> Box<dyn Stream<Item = Self::Block, Error = Error<Self::Error>> + Send>;
}
