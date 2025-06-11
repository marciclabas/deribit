use tokio::{net::TcpStream, sync::{mpsc, oneshot}};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde::{Serialize};

use crate::core::{Error, Notification, Response, ResponseHandler};

pub const TESTNET: &str = "wss://test.deribit.com/ws/api/v2";
pub const MAINNET: &str = "wss://www.deribit.com/ws/api/v2";

#[derive(Debug, Serialize)]
pub struct JsonRpcRequest<'a> {
  pub jsonrpc: &'a str,
  pub id: u64,
  pub method: &'a str,
  pub params: serde_json::Value,
}


pub struct SocketClient {
  pub write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
  pub handler: ResponseHandler,
}

impl SocketClient {

  /// Start a new public client session with the given WebSocket stream.
  /// - `socket` - The WebSocket stream to use for communication.
  pub fn start(socket: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {

    let (write, mut read) = socket.split();
    let handler = ResponseHandler::new();
    let handler_clone = handler.clone();

    tokio::spawn(async move {
      while let Some(Ok(tungstenite::Message::Text(msg))) = read.next().await {
        handler_clone.handle(&msg);
      }
    });

    Self { write, handler }
  }
  
  /// Start an aunthenticated client session.
  /// - `url` - The WebSocket URL to connect to, e.g. `deribit::TESTNET` or `deribit::MAINNET`.
  pub async fn connect(url: &str) -> Result<Self, Error> {
    let (socket, _) = connect_async(url).await
    .map_err(|e| Error::WebSocket(e))?;
    Ok(Self::start(socket))
  }


  /// Send an unauthenticated request without waiting for the reply. For **public** methods only.
  /// - `method` - The API method to call, e.g. `"public/get_instruments"`
  /// - `params` - The parameters for the request, as a JSON object.
  pub async fn send(&mut self, method: &str, params: serde_json::Value, id: u64) -> Result<(), Error> {
    let msg = serde_json::to_string(&JsonRpcRequest { jsonrpc: "2.0", id, method, params })?;
    self.write.send(tungstenite::Message::Text(msg)).await?;
    Ok(())
  }

  /// Send an unauthenticated request and wait for its reply. For **public** methods only.
  /// - `method` - The API method to call, e.g. `"public/get_instruments"`
  /// - `params` - The parameters for the request, as a JSON object.
  pub async fn request(&mut self, method: &str, params: serde_json::Value) -> Result<Response, Error> {
    let (tx, rx) = oneshot::channel();
    let id = self.handler.request(tx);
    self.send(method, params, id).await?;
    Ok(rx.await?)
  }

  /// Register a listener for the specified channel. Actual subscription must be sent to the API separately.
  /// - `channel` - The channel ID to listen to
  /// - `sender` - notifications will be sent here
  pub fn listen(&self, channel: String, sender: mpsc::Sender<Notification>) {
    self.handler.subscribe(channel, sender);
  }
}