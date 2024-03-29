use bson::{bson, Bson};
use log::*;
use tempdir::TempDir;

mod implementations;

use implementations::{rpc_echo_listener::SimpleEchoListener, rpc_echo_sender::SimpleEchoSender};
use tokio_stream::StreamExt;

use crate::implementations::acc_monitor::AccMonitor;

#[tokio::test(flavor = "multi_thread")]
async fn test_rpc_calls() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let socket_dir = TempDir::new("karo_hub_socket_dir").expect("Failed to create socket tempdir");
    let socket_path: String = socket_dir
        .path()
        .join("karo_hub.socket")
        .as_os_str()
        .to_str()
        .unwrap()
        .into();

    let _listener = SimpleEchoListener::new(&socket_path).await;

    let monitor = Box::new(AccMonitor::default());
    let mut sender = SimpleEchoSender::new_with_monitor(&socket_path, monitor.clone()).await;

    let message = bson!({
        "message": "Hello world!"
    });

    // See logs for this one
    sender.send(&message).await;

    // One time response. See logs if removed response from call registry
    let call = sender.call(&message).await;

    debug!("Call response: {}", call.body::<Bson>());

    // Subscription. Test implementation will return 5 echoes
    let mut subscription = sender.subscribe(&message).await;
    debug!(
        "Subscription response 1: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 2: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 3: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 4: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 5: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );

    println!("{}", monitor.print().await);

    let monitor_messages = monitor.messages().lock().await;

    assert_eq!(monitor_messages.len(), 9);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_rpc_reconnect() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let socket_dir = TempDir::new("karo_hub_socket_dir").expect("Failed to create socket tempdir");
    let socket_path: String = socket_dir
        .path()
        .join("karo_hub.socket")
        .as_os_str()
        .to_str()
        .unwrap()
        .into();

    let mut listener = SimpleEchoListener::new(&socket_path).await;
    let mut sender = SimpleEchoSender::new(&socket_path).await;

    let message = bson!({
        "message": "Hello world!"
    });

    // Subscription. Test implementation will return 5 echoes
    let mut subscription = sender.subscribe(&message).await;
    debug!(
        "Subscription response 1: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 2: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 3: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 4: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Subscription response 5: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );

    // After we reconnect, sender resubscribers and listener should send another 5 reponses
    listener.restart().await;

    debug!(
        "Resubscription response 1: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Resubscription response 2: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Resubscription response 3: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Resubscription response 4: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
    debug!(
        "Resubscription response 5: {}",
        subscription.next().await.unwrap().body::<Bson>()
    );
}
