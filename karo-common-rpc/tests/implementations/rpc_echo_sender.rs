use bson::Bson;
use karo_common_connection::monitor::Monitor;
use karo_common_rpc::{rpc_connection::RpcConnection, rpc_sender::RpcSender, Message};
use log::*;
use tokio_stream::wrappers::ReceiverStream;

use crate::implementations::message_type::MessageType;

use super::simple_connector::SimpleConnector;

pub struct SimpleEchoSender {
    sender: RpcSender,
}

impl SimpleEchoSender {
    pub async fn new(socket_path: &str) -> Self {
        let connector = SimpleConnector::new(socket_path);
        let connection = RpcConnection::new(Box::new(connector)).await.unwrap();

        let sender = connection.sender();

        Self::start_loop(connection).await;

        Self { sender }
    }

    pub async fn new_with_monitor(socket_path: &str, monitor: Box<dyn Monitor>) -> Self {
        let connector = SimpleConnector::new(socket_path);
        let mut connection = RpcConnection::new(Box::new(connector)).await.unwrap();

        connection.set_monitor(monitor);

        let sender = connection.sender();

        Self::start_loop(connection).await;

        Self { sender }
    }

    pub async fn start_loop(mut connection: RpcConnection) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    message = connection.read() => {
                        unreachable!("Client shold never receive a message. Got: {:?}", message);
                    }
                }
            }
        });
    }

    pub async fn send(&mut self, message: &Bson) {
        debug!("Sending message: {:?}", message);

        let bson = bson::to_bson(&MessageType::Message(message.clone())).unwrap();

        self.sender.send(bson).await.unwrap();
    }

    pub async fn call(&mut self, message: &Bson) -> Message {
        debug!("Call: {:?}", message);

        let bson = bson::to_bson(&MessageType::Call(message.clone())).unwrap();

        self.sender.call(&bson).await.unwrap()
    }

    pub async fn subscribe(&mut self, message: &Bson) -> ReceiverStream<Message> {
        debug!("Subscription: {:?}", message);

        let bson = bson::to_bson(&MessageType::Subscription(message.clone())).unwrap();

        self.sender.subscribe(&bson).await.unwrap()
    }
}
