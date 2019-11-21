use kyansel::{cancellable, CancellableResult};
use std::future::Future;
use tokio::{
    prelude::*,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver},
        oneshot::{channel, error::RecvError, Sender},
    },
    timer::delay,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _) = channel();
    let mut tx = Some(tx);

    //this is here to collect the result at the end
    let (cancel_tx, cancel_rx) = unbounded_channel();
    let (finish_tx, finish_rx) = unbounded_channel();

    for i in 0..20 {
        let mut finish_tx = finish_tx.clone();
        let mut cancel_tx = cancel_tx.clone();
        //create cancellable future
        let (tmp_tx, fut) = create_cancellable_future(i, tx.take().unwrap());
        tx = Some(tmp_tx);
        tokio::spawn(async move {
            //complete the cancellable future
            let result = fut.await;

            //check the result and send it to one of the 2 collectors
            let _ = match result {
                CancellableResult::Cancelled(canceler) => {
                    cancel_tx.send((i, canceler.unwrap())).await
                }
                CancellableResult::Finished(me) => finish_tx.send(me).await,
            };
        });

        //delay between each spawn to see the effect of cancelling
        delay(tokio::clock::now() + std::time::Duration::from_millis(20)).await;
    }

    //drop the tx so the collectors finish
    std::mem::drop(cancel_tx);
    std::mem::drop(finish_tx);

    //collect all futures that finished
    async move {
        let mut v = collect_channel::<usize>(finish_rx).await;
        v.sort();

        println!("Finished: {:?}", v);
    }
    .await;

    //collect all futures that were cancelled
    async move {
        let mut v = collect_channel::<(usize, usize)>(cancel_rx).await;
        v.sort();

        //(future number, canceler number)
        println!("Cancelled: {:?}", v);
    }
    .await;

    Ok(())
}

//collect items from a channel
async fn collect_channel<T>(mut rx: UnboundedReceiver<T>) -> Vec<T> {
    let mut v = Vec::new();

    while let Some(n) = rx.recv().await {
        v.push(n);
    }

    v
}

//get a future that can be cancelled by sending a message down the first element of the tuple
fn create_cancellable_future(
    input: usize,
    tx: Sender<usize>,
) -> (Sender<usize>, impl Future<Output = CancellableResult<usize, Result<usize, RecvError>>>) {
    let cancel_previous = async move { tx.send(input) };

    let (tx, rx) = channel();

    //simulate computation
    let fut = async move {
        delay(tokio::clock::now() + std::time::Duration::from_millis(20)).await;
        input
    };

    let cancel = cancellable(fut, rx);

    (tx, async move {
        let _ = cancel_previous.await;
        cancel.await
    })
}
