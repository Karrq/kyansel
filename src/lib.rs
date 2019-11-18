use futures::{Async, Future, Poll};

pub struct Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    inner: F,
    stopper: Option<S>,
}

#[derive(Debug)]
pub enum CancellableError<C, E> {
    Cancelled(C),
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
        let mut is_ready = false;

        let inner = match self.inner.poll() {
            Ok(ok @ Async::NotReady) => Ok(ok),
            Ok(ok @ Async::Ready(_)) => {
                is_ready = true;
                Ok(ok)
            }
            Err(e) => Err(CancellableError::Errored(e)),
        };

        if let Some(ref mut stopper) = self.stopper {
            match stopper.poll() {
                Ok(Async::Ready(s)) if !is_ready => return Err(CancellableError::Cancelled(s)),
                Ok(_) => {} //if is_ready, do nothing
                Err(_) => {
                    self.stopper = None;
                } //don't poll again
            }
        }

        inner
    }
}

pub trait FutureExt: Future {
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

impl<T: ?Sized> FutureExt for T where T: Future {}
