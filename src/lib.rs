#[cfg(feature = "futures_01")]
mod futures_01;
#[cfg(feature = "futures_01")]
pub use futures_01::*;

#[cfg(not(feature = "futures_01"))]
mod futures_std;
#[cfg(not(feature = "futures_01"))]
pub use futures_std::*;
