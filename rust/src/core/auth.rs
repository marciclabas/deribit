use serde::{Deserialize, Serialize};
use crate::core::{client::LogLevel, parse_json, DeribitResponse, Error, PublicClient};

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
  pub scope: String,
  pub sid: Option<String>,
  pub token_type: String,
}

#[derive(Debug, Clone)]
pub struct Auth {
  pub response: AuthResponse,
  pub expires_at: std::time::Instant,
}

impl Auth {
  pub fn new(response: AuthResponse) -> Self {
    let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(response.expires_in as u64);
    Self { response, expires_at }
  }

  pub fn expired(&self) -> bool {
    self.expires_at <= std::time::Instant::now()
  }
}

impl PublicClient {
  pub async fn authenticate(&mut self, client_id: &str, client_secret: &str) -> Result<Auth, Error> {
    let params = serde_json::json!({
      "grant_type": "client_credentials",
      "client_id": client_id,
      "client_secret": client_secret,
    });
    let resp = self.request("public/auth", params).await?.value()?;
    let auth = parse_json::<AuthResponse>(resp)?;
    Ok(Auth::new(auth))
  }

  pub async fn refresh_token(&mut self, refresh_token: &str) -> Result<Auth, Error> {
    let params = serde_json::json!({
      "grant_type": "refresh_token",
      "refresh_token": refresh_token,
    });
    let resp = self.request("public/auth", params).await?.value()?;
    let auth = parse_json::<AuthResponse>(resp)?;
    Ok(Auth::new(auth))
  }
}

pub struct PrivateClient {
  pub client: PublicClient,
  pub auth: Auth,
}

impl PrivateClient {
  /// Authenticate an existing public client session.
  /// - `client_id` - The client ID provided by Deribit.
  /// - `client_secret` - The client secret provided by Deribit.
  ///
  /// Source: [Deribit docs](https://docs.deribit.com/#public-auth)
  pub async fn authenticate(
    client_id: &str,
    client_secret: &str,
    mut client: PublicClient,
  ) -> Result<Self, Error> {
    let auth = client.authenticate(client_id, client_secret).await?;
    Ok(Self { client, auth })
  }

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
  ) -> Result<Self, Error> {
    let client = PublicClient::connect(url).await?;
    Self::authenticate(client_id, client_secret, client).await
  }
  
  /// Start a new authenticated client session with debug mode enabled.
  /// - `url` - The URL of the Deribit API, e.g. `deribit::MAINNET` or `deribit::TESTNET`.
  /// - `client_id` - The client ID provided by Deribit.
  /// - `client_secret` - The client secret provided by Deribit.
  ///
  /// Source: [Deribit docs](https://docs.deribit.com/#public-auth)
  pub async fn start_debug(
    url: &str,
    client_id: &str,
    client_secret: &str,
    log: LogLevel,
  ) -> Result<Self, Error> {
    let client = PublicClient::connect_debug(url, log).await?;
    Self::authenticate(client_id, client_secret, client).await
  }

  /// Send an unauthenticated request. For **public** methods only.
  /// - `method` - The API method to call, e.g. `"public/get_instruments"`
  /// - `params` - The parameters for the request, as a JSON object.
  pub async fn request(&mut self, method: &str, params: serde_json::Value) -> Result<DeribitResponse, Error> {
    self.client.request(method, params).await
  }

  /// Refresh the current access token using the stored refresh token.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#public-auth)
  pub async fn refresh_token(&mut self) -> Result<&Auth, Error> {
    self.auth = self.client.refresh_token(&self.auth.response.refresh_token).await?;
    Ok(&self.auth)
  }

  /// Send an authenticated request using the current access token.
  pub async fn authed_request(&mut self, method: &str, params: serde_json::Value) -> Result<DeribitResponse, Error> {
    if self.auth.expired() {
      self.refresh_token().await?;
    }
    let mut params = params;
    params["access_token"] = serde_json::Value::String(self.auth.response.access_token.to_string());
    self.request(method, params).await
  }

  /// Exchanges the current access token for a subaccount's token.
  /// - `subject_id` - The ID of the subaccount to exchange the token for. Can be found on https://deribit.com/account/BTC/subaccounts.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#public-exchange_token)
  pub async fn exchange_token(&mut self, subject_id: i64) -> Result<Auth, Error> {
    let params = serde_json::json!({
      "refresh_token": self.auth.response.refresh_token,
      "subject_id": subject_id,
    });
    let val = self.request("public/exchange_token", params).await?.value()?;
    let auth = parse_json::<AuthResponse>(val)?;
    Ok(Auth::new(auth))
  }

  /// Switches the current authentication context to a subaccount
  /// - `subject_id` - The ID of the subaccount to switch to. Can be found on https://deribit.com/account/BTC/subaccounts.
  /// 
  /// Source: [Deribit docs](https://docs.deribit.com/#public-exchange_token)
  pub async fn switch_subaccount(&mut self, subject_id: i64) -> Result<&Auth, Error> {
    self.auth = self.exchange_token(subject_id).await?;
    Ok(&self.auth)
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
    self.client.send("private/logout", params, 0).await?; // the server doesn't reply to this method
    Ok(())
  }
}