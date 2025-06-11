use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use serde::{Deserialize};

use crate::core::{ApiError, Error};

/// JSON-RPC response, with either a result or an error.
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
  pub jsonrpc: String,
  pub id: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<ApiError>,
}

/// JSON-RPC notification
#[derive(Debug, Clone, Deserialize)]
pub struct Notification {
  pub jsonrpc: String,
  pub params: NotificationParams
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationParams {
  pub channel: String,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Message {
  Response(Response),
  Notification(Notification),
}

impl Response {
  pub fn value(&self) -> Result<serde_json::Value, Error> {
    if let Some(ref result) = self.result {
      Ok(result.clone())
    } else if let Some(ref error) = self.error {
      Err(Error::Api(error.clone()))
    } else {
      Err(Error::Logic("Response must contain either result or error"))
    }
  }
}

#[derive(Debug, Clone)]
pub struct ResponseHandler {
  pub id_counter: u64,
  pub requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Response>>>>,
  pub subscriptions: Arc<Mutex<HashMap<String, mpsc::Sender<Notification>>>>,
}

impl ResponseHandler {
  pub fn new() -> Self {
    ResponseHandler {
      id_counter: 0,
      requests: Arc::new(Mutex::new(HashMap::new())),
      subscriptions: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  pub fn handle(&self, message: &str) {
    match serde_json::from_str::<Message>(message) {
      Ok(Message::Response(resp)) => {
        let mut requests = self.requests.lock().unwrap();
        if let Some(sender) = requests.remove(&resp.id) {
          let _ = sender.send(resp);
        }
      }
      Ok(Message::Notification(notif)) => {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        if let Some(sender) = subscriptions.get_mut(&notif.params.channel) {
          let _ = sender.send(notif);
        }
      }
      Err(e) => {
        eprintln!("Failed to parse message: {}", e);
      }
    }
  }

  pub fn subscribe(&self, channel: String, sender: mpsc::Sender<Notification>) {
    let mut subscriptions = self.subscriptions.lock().unwrap();
    subscriptions.insert(channel, sender);
  }

  pub fn unsubscribe(&self, channel: &str) {
    let mut subscriptions = self.subscriptions.lock().unwrap();
    subscriptions.remove(channel);
  }

  pub fn request(&mut self, sender: oneshot::Sender<Response>) -> u64 {
    self.id_counter += 1;
    let mut requests = self.requests.lock().unwrap();
    requests.insert(self.id_counter, sender);
    self.id_counter
  }
}

