use super::Cancellable;
use core::{future::Future, pin::Pin};

pub struct Projection<'pin, F, S>
where
    F: Future,
    S: Future,
{
    pub inner: Pin<&'pin mut F>,
    pub stopper: Pin<&'pin mut S>,
}

impl<F, S> Cancellable<F, S>
where
    F: Future,
    S: Future,
{
    pub(crate) fn project(self: Pin<&mut Self>) -> Projection<F, S> {
        unsafe {
            let this = self.get_unchecked_mut();
            Projection {
                inner: Pin::new_unchecked(&mut this.inner),
                stopper: Pin::new_unchecked(&mut this.stopper),
            }
        }
    }
}
