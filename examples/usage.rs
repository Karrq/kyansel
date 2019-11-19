#[cfg(feature = "futures_01")]
fn main() {
    use futures01::future::{Future, IntoFuture};
    use kyansel::FutureCancellable;
    use tokio01::{sync::oneshot, timer::Delay};

    let (tx, rx) = oneshot::channel::<()>();

    tokio01::run(
        Delay::new(tokio::clock::now() + std::time::Duration::from_secs(1).into())
            .map_err(|_| ())
            .and_then(|_| Ok(()))
            .cancel_with(|| rx)
            .map_err(|e| println!("{:?}", e))
            .join(tx.send(()).into_future())
            .and_then(|(_ok, _tx_send)| Ok(println!("unreachable"))),
    );
}

#[cfg(not(feature = "futures_01"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use futures::future::{join, ready};
    use kyansel::FutureCancellable;
    use tokio::{sync::oneshot, timer::delay};

    let (tx, rx) = oneshot::channel::<()>();

    let cancellable =
        delay(tokio::clock::now() + std::time::Duration::from_secs(1)).cancel_with(|| rx);

    let canceller = ready(tx.send(()));

    let pair = join(cancellable, canceller);

    let (cancellable_result, _) = pair.await;

    println!("{:?}", cancellable_result);

    Ok(())
}
