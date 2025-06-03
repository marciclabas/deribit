use std::{collections::HashMap, sync::Arc};

use futures_util::{lock::Mutex, stream::SplitSink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{net::TcpStream, sync::oneshot};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

use crate::core::{ApiError, Error};

pub const TESTNET: &str = "wss://test.deribit.com/ws/api/v2";
pub const MAINNET: &str = "wss://www.deribit.com/ws/api/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
  None = 0,
  Error = 1,
  Warning = 2,
  Info = 3,
  Debug = 4,
  Trace = 5,
}

/// Represents a full JSON-RPC response, with either a result or an error.
#[derive(Debug, Clone, Deserialize)]
pub struct DeribitResponse {
  pub jsonrpc: String,
  pub id: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<ApiError>,
}

impl DeribitResponse {
  pub fn value(&self) -> Result<Value, Error> {
    if let Some(ref result) = self.result {
      Ok(result.clone())
    } else if let Some(ref error) = self.error {
      Err(Error::Api(error.clone()))
    } else {
      Err(Error::Logic("Response must contain either result or error".to_string()))
    }
  }
}

#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
  pub jsonrpc: String,
  pub id: u64,
  pub method: String,
  pub params: serde_json::Value,
}

pub struct PublicClient {
  pub log: LogLevel,
  pub id: u64,
  pub write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
  pub subscribers: Arc<Mutex<HashMap<u64, oneshot::Sender<DeribitResponse>>>>,
}

impl PublicClient {

  pub fn start_impl(socket: WebSocketStream<MaybeTlsStream<TcpStream>>, log: LogLevel) -> Self {

    let (write, mut read) = socket.split();
    let subscribers: Arc<Mutex<HashMap<u64, oneshot::Sender<DeribitResponse>>>>
      = Arc::new(Mutex::new(HashMap::new()));

    let subs_clone = subscribers.clone();
    tokio::spawn(async move {
      while let Some(Ok(Message::Text(txt))) = read.next().await {
        if let Ok(resp) = serde_json::from_str::<DeribitResponse>(&txt) {
          let mut subs = subs_clone.lock().await;
          if let Some(sender) = subs.remove(&resp.id) {
            if log >= LogLevel::Debug {
              println!("Received response: {:?}", resp);
            }
            let _ = sender.send(resp);
          }
          else if log >= LogLevel::Warning {
            println!("No subscriber for response ID {}: {:?}", resp.id, resp);
          }
        }
        else if log >= LogLevel::Error {
          println!("Failed to parse response: {}", txt);
        }
      }
    });

    Self { id: 0, write, subscribers, log }
  }

  /// Start a new public client session with the given WebSocket stream.
  /// - `socket` - The WebSocket stream to use for communication.
  pub fn start(socket: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
    Self::start_impl(socket, LogLevel::None)
  }

  /// Start a new public client session with the given WebSocket stream and debug mode.
  /// - `socket` - The WebSocket stream to use for communication.
  /// - `log` - log level (higher is more verbose).
  pub fn start_debug(socket: WebSocketStream<MaybeTlsStream<TcpStream>>, log: LogLevel) -> Self {
    Self::start_impl(socket, log)
  }

  pub async fn connect_impl(url: &str, log: LogLevel) -> Result<Self, Error> {
    if log >= LogLevel::Info {
      println!("[INFO] Connecting to WebSocket to {}", url);
    }
    let (socket, _) = connect_async(url).await
    .map_err(|e| Error::WebSocket(e))?;
    Ok(Self::start_impl(socket, log))
  }

  /// Start an aunthenticated client session.
  /// - `url` - The WebSocket URL to connect to, e.g. `deribit::TESTNET` or `deribit::MAINNET`.
  pub async fn connect(url: &str) -> Result<Self, Error> {
    Self::connect_impl(url, LogLevel::None).await
  }

  /// Start an aunthenticated client session with debug mode enabled.
  /// - `url` - The WebSocket URL to connect to, e.g. `deribit::TESTNET` or `deribit::MAINNET`.
  /// - `log` - log level (higher is more verbose).
  pub async fn connect_debug(url: &str, log: LogLevel) -> Result<Self, Error> {
    Self::connect_impl(url, log).await
  }

  /// Send an unauthenticated request without waiting for the reply. For **public** methods only.
  /// - `method` - The API method to call, e.g. `"public/get_instruments"`
  /// - `params` - The parameters for the request, as a JSON object.
  pub async fn send(&mut self, method: &str, params: serde_json::Value, id: u64) -> Result<(), Error> {
    let msg = serde_json::to_string(&JsonRpcRequest {
      jsonrpc: "2.0".to_string(),
      id,
      method: method.to_string(),
      params,
    })?;
    if self.log >= LogLevel::Debug {
      println!("[DEBUG] Sending message with ID: {}", id);
    }
    else if self.log >= LogLevel::Trace {
      println!("[TRACE] Sending message: {}", msg);
    }
    self.write.send(Message::Text(msg)).await?;
    Ok(())
  }

  /// Send an unauthenticated request and wait for its reply. For **public** methods only.
  /// - `method` - The API method to call, e.g. `"public/get_instruments"`
  /// - `params` - The parameters for the request, as a JSON object.
  pub async fn request(&mut self, method: &str, params: serde_json::Value) -> Result<DeribitResponse, Error> {
    self.id += 1;
    let (tx, rx) = oneshot::channel();
    {
      let mut subs = self.subscribers.lock().await;
      subs.insert(self.id, tx);
    }
    self.send(method, params, self.id).await?;
    Ok(rx.await?)
  }
}