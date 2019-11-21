//! Cancellable future for futures 0.1
//!
//! # Example
//! ```rust
//! # fn main() {
//! #  use futures_01::future::{Future, IntoFuture};
//!  use kyansel::futures_01::{CancellableError, FutureCancellable};
//! #  use tokio_01::{sync::oneshot, timer::Delay, runtime::Builder};
//! #  fn run(fut: impl Future<Item = (), Error = ()> + Send + 'static) { Builder::new().panic_handler(|err| std::panic::resume_unwind(err)).build().unwrap().block_on(fut); }
//!
//!  let (tx, rx) = oneshot::channel::<()>();
//!
//!  let deadline = tokio::clock::now() + std::time::Duration::from_secs(1).into();
//!
//!  //simulate a long-running future
//!  let cancellable = Delay::new(deadline)
//!      .map_err(|_| ())
//!      .and_then(|_| Ok(()))
//!      //add a way to cancel it
//!      .cancel_with(rx)
//!      .map_err(|e| {
//!         assert_eq!(e, CancellableError::Cancelled(()));
//!       })
//!      .map_err(|_| ());
//!
//!  let canceller = tx.send(()).into_future();
//!
//!  run(
//!    //join it with the canceller
//!    cancellable.join(canceller)
//!    //it will cancel immediately returning err
//!    .and_then(|(_ok, _tx_send)| Ok(println!("unreachable"))),
//!  );
//! # }
//! ```

use futures_01::{Async, Future, Poll};

///Future for the [`cancel_with`](trait.FutureCancellable.html#method.cancel_with) combinator,
///allowing a computation to be cancelled if a second computation completes succesfully.
///
///If the future is cancelled it will complete as a
/// [`CancellableError::Cancelled`](enum.CancellableError.html#variant.Cancelled)
///
///Created with [`FutureCancellable::cancel_with`](trait.FutureCancellable.html#method.cancel_with)
/// or [`cancellable`](fn.cancellable.html)
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    inner: F,
    stopper: Option<S>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
///Error returned by [`Cancellable`](struct.Cancellable.html)
pub enum CancellableError<C, E> {
    ///If the inner future was cancelled
    Cancelled(C),

    ///If the inner future just errored
    Errored(E),
}

impl<C, E> CancellableError<C, E> {
    ///Check if the future was cancelled
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled(_) => true,
            _ => false,
        }
    }

    ///Retrieve the error of the future
    /// if it was not cancelled and it errored
    pub fn errored(self) -> Option<E> {
        match self {
            Self::Errored(e) => Some(e),
            _ => None,
        }
    }

    ///Retrieve the result of the canceller future
    /// if the future was cancelled
    pub fn cancelled(self) -> Option<C> {
        match self {
            Self::Cancelled(c) => Some(c),
            _ => None,
        }
    }
}

impl<F, S> Future for Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    type Error = CancellableError<S::Item, F::Error>;
    type Item = F::Item;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        //always poll inner future
        let inner = match self.inner.poll() {
            Ok(ok @ Async::NotReady) => Ok(ok),
            Ok(ok @ Async::Ready(_)) => {
                return Ok(ok); //return early with the result
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
        Cancellable { inner: self, stopper: Some(stopper) }
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
    Cancellable { inner, stopper: Some(stopper) }
}

impl<T: ?Sized> FutureCancellable for T where T: Future {}
