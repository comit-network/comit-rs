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
    /// source learns about them.
    fn blocks(&self) -> Box<dyn Stream<Item = Self::Block, Error = Error<Self::Error>> + Send>;
}
