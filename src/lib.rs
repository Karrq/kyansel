use futures::{Async, Future, Poll};

///Future for the `cancel_with` combinator, allowing a computation to be cancelled
///if a second computation completes succesfully.
///
///Created with [`FutureCancellable::cancel_with`](trait.FutureCancellable.html#method.cancel_with)
pub struct Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    inner: F,
    stopper: Option<S>,
}

#[derive(Debug)]
///Error returned by [`Cancellable`](struct.Cancellable.html)
pub enum CancellableError<C, E> {
    ///If the inner future was cancelled
    Cancelled(C),
    
    ///If the inner future just errored
    Errored(E),
}

impl<F, S> Future for Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    type Item = F::Item;
    type Error = CancellableError<S::Item, F::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        
        //always poll inner future
        let inner = match self.inner.poll() {
            Ok(ok @ Async::NotReady) => Ok(ok),
            Ok(ok @ Async::Ready(_)) => {
                return Ok(ok)   //return early with the result
            }
            Err(e) => Err(CancellableError::Errored(e)),
        };

        if let Some(ref mut stopper) = self.stopper {
            match stopper.poll() {
                //if the inner future was ready we won't reach this
                Ok(Async::Ready(s)) => return Err(CancellableError::Cancelled(s)), 
                Ok(_) => {}
                Err(_) => {
                    //don't poll again
                    self.stopper = None;
                } 
            }
        }
        
        //this is either Ok(NotReady) or Err(Errored)
        inner
    }
}

/// An extension trait for `Future` that provides the [`Cancellable`](struct.Cancellable.html) combinator.
///
/// Users are not expected to implement this trait. All types that implement
/// `Future` already implement `FutureCancellable`.
pub trait FutureCancellable: Future {
    ///Cancel this future if another one completes succesfully
    ///
    ///Note that this function consumes the receiving future and returns a wrapped version of it
    ///
    /// # Examples
    /// An example can be found in `examples/usage.rs`
    fn cancel_with<S>(self, stopper: S) -> Cancellable<Self, S>
    where
        S: Future,
        Self: Sized,
    {
        Cancellable {
            inner: self,
            stopper: Some(stopper),
        }
    }
}

impl<T: ?Sized> FutureCancellable for T where T: Future {}
