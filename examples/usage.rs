use futures::future::{Future, IntoFuture};
use rax::FutureExt;
use tokio::{sync::oneshot, timer::Delay};

fn main() {
    let (tx, rx) = oneshot::channel::<()>();

    let r = tokio::run(
        Delay::new(tokio::clock::now() + std::time::Duration::from_secs(1).into())
            .map_err(|_| ())
            .and_then(|_| Ok(()))
            .cancel_with(rx)
            .map_err(|e| println!("cancellable errored: {:?}", e))
            .join(tx.send(()).into_future())
            .and_then(|(_ok, _tx_send)| Ok(println!("both succesfull"))),
    );

    println!("{:?}", r);
}
