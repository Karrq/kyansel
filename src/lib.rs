//!This library adds a combinator for futures, enabling a future to be
//! cancelled when another one has completed succesfully.
//!
//!Support for futures 0.1 can be enabled with the `futures_01` feature
//!
//! # Example
//! ```rust
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #    use futures::future::{join, ready};
//!  use kyansel::cancellable;
//! #    use tokio::{sync::oneshot, timer::delay};
//!
//!  let (tx, rx) = oneshot::channel::<()>();
//!     
//!  //simulate a long future
//!  let future =
//!      delay(tokio::clock::now() + std::time::Duration::from_secs(1));
//!
//!  //make it cancellable
//!  let cancellable = cancellable(future, rx);
//!
//!  //create the future that will trigger the cancellation
//!  let canceller = ready(tx.send(()));
//!
//!  //run them at the same time (example)
//!  let pair = join(cancellable, canceller);
//!
//!  //we `.await` the join, dropping the result of the canceller since we don't care
//!  let (cancellable_result, _) = pair.await;
//!
//!  //the return is of type CancelledResult<(), Result<(), RecvError>>
//!  let cancellable_result = cancellable_result.cancelled().unwrap().unwrap();
//!
//!  assert_eq!((), cancellable_result);
//! #
//! # Ok(())
//! # }
//! ```

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "futures_01")]
pub mod futures_01;

mod projection;

///Future for the [`cancel_with`](trait.FutureCancellable.html#method.cancel_with) combinator,
///allowing a computation to be cancelled if a second computation completes succesfully.
///
///If the future is cancelled it will complete as a
/// [`CancellableError::Cancelled`](enum.CancellableError.html#variant.Cancelled)
///
///Created with [`FutureCancellable::cancel_with`](trait.FutureCancellable.html#method.cancel_with)
/// or [`cancellable`](fn.cancellable.html)
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    inner: F,
    stopper: S,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
///Result returned by [`Cancellable`](struct.Cancellable.html)
pub enum CancellableResult<T, S> {
    ///If the inner future finished
    Finished(T),

    ///If the inner future was cancelled
    Cancelled(S),
}

impl<T, S> CancellableResult<T, S> {
    ///Check if the future was cancelled
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled(_) => true,
            _ => false,
        }
    }

    ///Retrieve the result of the future
    /// if it was not cancelled
    pub fn finished(self) -> Option<T> {
        match self {
            Self::Finished(f) => Some(f),
            _ => None,
        }
    }

    ///Retrieve the result of the canceller future
    /// if the future was cancelled
    pub fn cancelled(self) -> Option<S> {
        match self {
            Self::Cancelled(s) => Some(s),
            _ => None,
        }
    }
}

impl<F, S> Future for Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    type Output = CancellableResult<F::Output, S::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();

        //always poll inner future first
        match this.inner.poll(cx) {
            Poll::Pending => {}
            Poll::Ready(ready) => {
                //return early with the result
                return Poll::Ready(CancellableResult::Finished(ready));
            }
        };

        match this.stopper.poll(cx) {
            //if the inner future was ready we won't reach this
            Poll::Ready(s) => return Poll::Ready(CancellableResult::Cancelled(s)),
            Poll::Pending => {}
        };

        //if we were Ready at any point we won't reach this
        Poll::Pending
    }
}

/// An extension trait for `Future` that provides the [`Cancellable`](struct.Cancellable.html)
/// combinator.
///
/// Users are not expected to implement this trait. All types that implement
/// `Future` already implement `FutureCancellable`.
pub trait FutureCancellable: Future {
    ///Cancel this future if another one completes succesfully
    ///
    ///Note that this function consumes the receiving future and returns a wrapped version of it
    fn cancel_with<S>(self, stopper: S) -> Cancellable<Self, S>
    where
        S: Future,
        Self: Sized,
    {
        Cancellable { inner: self, stopper }
    }
}

///Creates a new [`Cancellable`](struct.Cancellable.html)
///
///This is essentially the same as
/// [`FutureCancellable::cancel_with`](trait.FutureCancellable.html#method.cancel_with),
/// the difference being that this is a function instead of a method
pub fn cancellable<Fut1, Fut2>(inner: Fut1, stopper: Fut2) -> Cancellable<Fut1, Fut2>
where
    Fut1: Future,
    Fut2: Future,
{
    Cancellable { inner, stopper }
}

impl<T: ?Sized> FutureCancellable for T where T: Future {}
