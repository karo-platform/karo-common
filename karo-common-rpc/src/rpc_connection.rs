use std::sync::Arc;

use anyhow::{Context, Result};
use bson;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

use karo_common_connection::{connection::Connection, connector::Connector};

use crate::{
    call_registry::CallRegistry,
    message::{Message, MessageType},
    rpc_connector::RpcConnector,
    rpc_sender::RpcSender,
    user_message::UserMessageHandle,
};

/// RPC connection handle.
/// Uses [CallRegistry] to account user calls.
/// Uses [Connector] wrapper to resubscribe if reconnected
pub struct RpcConnection<S: AsyncReadExt + AsyncWriteExt> {
    /// Socket connection to send/receive data
    connection: Connection<S>,
    /// Common sender, which can be used to clone and return to a user
    sender: RpcSender,
    /// Call registry, which is used to record calls, resubscribe on reconnection and send user responses
    call_registry: Arc<Mutex<CallRegistry>>,
}

impl<S: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static> RpcConnection<S> {
    /// Contructor. Uses [Connector] to connect to the peer
    pub async fn new(connector: Box<dyn Connector<S>>) -> Result<Self> {
        let call_registry = Arc::new(Mutex::new(CallRegistry::new()));

        let rpc_connector = RpcConnector::new(connector, call_registry.clone());

        let connection = Connection::new(Box::new(rpc_connector)).await?;
        let sender = RpcSender::new(connection.writer(), call_registry.clone());

        Ok(Self {
            connection,
            sender,
            call_registry,
        })
    }

    /// Read incoming messages
    pub async fn read(&mut self) -> Result<UserMessageHandle> {
        // The function loops if received a response. In this case we send the reponse
        // to a user using a future from the call registry and read nex message.
        loop {
            let incoming_bson = self.connection.read_bson().await?;

            let incoming_message = bson::from_bson::<Message>(incoming_bson)
                .context("Failed to deserialize incoming message")?;

            match incoming_message.message_type {
                MessageType::Call => {
                    return Ok(UserMessageHandle::new_call(
                        incoming_message,
                        self.connection.writer(),
                    ))
                }
                MessageType::Message => return Ok(UserMessageHandle::new(incoming_message)),
                MessageType::Response => {
                    self.call_registry
                        .lock()
                        .await
                        .resolve(UserMessageHandle::new(incoming_message))
                        .await
                }
            }
        }
    }

    pub fn sender(&self) -> RpcSender {
        self.sender.clone()
    }

    /// Reset all existing calls on reconnect
    pub async fn reset(&mut self) {
        self.call_registry.lock().await.clear()
    }
}
