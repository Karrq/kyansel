use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use pin_project_lite::pin_project;

pin_project! {
    ///Future for the `cancel_with` combinator, allowing a computation to be cancelled
    ///if a second computation completes succesfully.
    ///
    ///Created with [`FutureCancellable::cancel_with`](trait.FutureCancellable.html#method.cancel_with)
    #[derive(Debug)]
    #[must_use = "futures do nothing unless polled"]
    pub struct Cancellable<F, S>
    where
        F: Future,
        S: Future,
    {
        #[pin]
        inner: F,
        #[pin]
        stopper: S,

    }
}

#[derive(Debug)]
///Result returned by [`Cancellable`](struct.Cancellable.html)
pub enum CancellableResult<T, S> {
    ///If the inner future finished
    Finished(T),

    ///If the inner future was cancelled
    Cancelled(S),
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
    ///
    /// # Examples
    /// An example can be found in `examples/usage.rs`
    fn cancel_with<S>(self, stopper: impl FnOnce() -> S) -> Cancellable<Self, S>
    where
        S: Future,
        Self: Sized,
    {
        Cancellable { inner: self, stopper: (stopper)() }
    }
}

impl<T: ?Sized> FutureCancellable for T where T: Future {}
