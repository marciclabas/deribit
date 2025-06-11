use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use crate::core::{parse_json, Error, SocketClient, Response, Scope};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
  pub access_token: String,
  #[serde(default)]
  pub enabled_features: Vec<String>,
  pub expires_in: i64,
  #[serde(default)]
  pub google_login: bool,
  #[serde(default)]
  pub mandatory_tfa_status: String,
  pub refresh_token: String,
  pub scope: Scope,
  pub sid: Option<String>,
  pub token_type: String,
}

#[derive(Debug, Clone)]
pub struct Auth {
  pub response: AuthResponse,
  pub expires_at: std::time::Instant,
}

impl AuthResponse {
  pub fn parse(self) -> Auth {
    let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(self.expires_in as u64);
    Auth { response: self, expires_at }
  }
}

impl Auth {
  pub fn expired(&self) -> bool {
    self.expires_at <= std::time::Instant::now()
  }
}

impl SocketClient {
  /// Authenticate an existing public client session. Returns the authentication details; use `authenticated` to get a `PrivateClient`.
  /// - `client_id` - The client ID provided by Deribit.
  /// - `client_secret` - The client secret provided by Deribit.
  /// - `scope` - The scope of the new private session, e.g. `deribit::Scope::default()`.
  ///
  /// Source: [Deribit docs](https://docs.deribit.com/#public-auth)
  pub async fn authenticate(&mut self, client_id: &str, client_secret: &str, scope: Scope) -> Result<Auth, Error> {
    let params = serde_json::json!({
      "grant_type": "client_credentials",
      "client_id": client_id,
      "client_secret": client_secret,
      "scope": scope.dump(),
    });
    let resp = self.request("public/auth", params).await?.value()?;
    let auth = parse_json::<AuthResponse>(resp)?.parse();
    Ok(auth)
  }

  /// Authenticate an existing public client session. The returned client can be used to make authenticated requests.
  /// - `client_id` - The client ID provided by Deribit.
  /// - `client_secret` - The client secret provided by Deribit.
  /// - `scope` - The scope (i.e. permissions) of the new private session, e.g. `deribit::Scope::default()`.
  ///
  /// Source: [Deribit docs](https://docs.deribit.com/#public-auth)
  pub async fn authenticated(mut self, client_id: &str, client_secret: &str, scope: Scope) -> Result<PrivateClient, Error> {
    let auth = self.authenticate(client_id, client_secret, scope).await?;
    let client = Arc::new(Mutex::new(self));
    Ok(PrivateClient { client, auth })
  }
}

pub struct PrivateClient {
  pub client: Arc<Mutex<SocketClient>>,
  pub auth: Auth,
}

impl PrivateClient {
  /// Start a new authenticated client session.
  /// - `url` - The URL of the Deribit API, e.g. `deribit::MAINNET` or `deribit::TESTNET`.
  /// - `client_id` - The client ID provided by Deribit.
  /// - `client_secret` - The client secret provided by Deribit.
  ///
  /// Source: [Deribit docs](https://docs.deribit.com/#public-auth)
  pub async fn start(
    url: &str,
    client_id: &str,
    client_secret: &str,
    scope: Scope,
  ) -> Result<Self, Error> {
    let client = SocketClient::connect(url).await?;
    client.authenticated(client_id, client_secret, scope).await
  }
  
  /// Send an unauthenticated request. For **public** methods only.
  /// - `method` - The API method to call, e.g. `"public/get_instruments"`
  /// - `params` - The parameters for the request, as a JSON object.
  pub async fn request(&mut self, method: &str, params: serde_json::Value) -> Result<Response, Error> {
    self.client.as_ref().lock().unwrap().request(method, params).await
  }

  /// Send an unauthenticated message without waiting for a response.
  /// This is useful for methods that don't return a response, like `private/logout`.
  /// - `method` - The API method to call, e.g. `"private/logout"`
  /// - `params` - The parameters for the request, as a JSON object.
  /// - `id` - The ID of the request. This is generally used to match the response to the request.
  pub async fn send(&mut self, method: &str, params: serde_json::Value, id: u64) -> Result<(), Error> {
    self.client.as_ref().lock().unwrap().send(method, params, id).await
  }

  /// Refresh the current access token using the stored refresh token.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#public-auth)
  pub async fn refresh_token(&mut self) -> Result<&Auth, Error> {
    let params = serde_json::json!({
      "grant_type": "refresh_token",
      "refresh_token": self.auth.response.refresh_token,
    });
    let resp = self.request("public/auth", params).await?.value()?;
    self.auth = parse_json::<AuthResponse>(resp)?.parse();
    Ok(&self.auth)
  }

  /// Send an authenticated request using the current access token.
  pub async fn authed_request(&mut self, method: &str, params: serde_json::Value) -> Result<Response, Error> {
    if self.auth.expired() {
      self.refresh_token().await?;
    }
    let mut params = params;
    params["access_token"] = serde_json::Value::String(self.auth.response.access_token.to_string());
    self.request(method, params).await
  }

  /// Exchanges the current access token for a subaccount's token. Doesn't change the current authentication context; use `swtich_subaccount` for that.
  /// - `subject_id` - The ID of the subaccount to exchange the token for. Can be found on https://deribit.com/account/BTC/subaccounts.
  /// - `scope` - Optional scope to request. Permissions cannot exceed those of the current session.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#public-exchange_token)
  pub async fn exchange_token(&mut self, subject_id: i64, scope: Option<Scope>) -> Result<Auth, Error> {
    let mut params = serde_json::json!({
      "refresh_token": self.auth.response.refresh_token,
      "subject_id": subject_id,
    });
    if let Some(scope) = scope {
      params["scope"] = serde_json::Value::String(scope.dump());
    }
    let val = self.request("public/exchange_token", params).await?.value()?;
    let auth = parse_json::<AuthResponse>(val)?.parse();
    Ok(auth)
  }

  /// Switches the current authentication context to a subaccount
  /// - `subject_id` - The ID of the subaccount to switch to. Can be found on https://deribit.com/account/BTC/subaccounts.
  /// - `scope` - Optional scope to request. Permissions cannot exceed those of the current session.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#public-exchange_token)
  pub async fn switch_subaccount(&mut self, subject_id: i64, scope: Option<Scope>) -> Result<&Auth, Error> {
    self.auth = self.exchange_token(subject_id, scope).await?;
    Ok(&self.auth)
  }

  /// Forks the current access token to a new session with the given name. Doesn't change the current authentication context; use `fork_session` for that.
  /// - `session_name` - The name of the new session. This can be any nonempty string, but should be unique for each session.
  ///
  /// Source: [Deribit docs](https://docs.deribit.com/#public-fork_token)
  pub async fn fork_token(&mut self, session_name: &str) -> Result<Auth, Error> {
    let params = serde_json::json!({
      "refresh_token": self.auth.response.refresh_token,
      "session_name": session_name,
    });
    let val = self.request("public/fork_token", params).await?.value()?;
    let auth = parse_json::<AuthResponse>(val)?.parse();
    Ok(auth)
  }

  /// Forks the current access token to a new session with the given name and returns a new `PrivateClient` with the new session.
  /// - `session_name` - The name of the new session. This can be any nonempty string, but should be unique for each session.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#public-fork_token)
  pub async fn fork_session(&mut self, session_name: &str) -> Result<PrivateClient, Error> {
    let auth = self.fork_token(session_name).await?;
    let client = Arc::clone(&self.client);
    Ok(PrivateClient { client, auth })
  }

  /// Gracefully closes the connection.
  /// - `invalidate_token` - If true, the access token will be invalidated.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#private-logout)
  pub async fn logout(&mut self, invalidate_token: bool) -> Result<(), Error> {
    let params = serde_json::json!({
      "invalidate_token": invalidate_token,
      "access_token": self.auth.response.access_token,
    });
    self.send("private/logout", params, 0).await?; // the server doesn't reply to this method
    Ok(())
  }
}