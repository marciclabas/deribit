use serde::{Deserialize, Serialize};


#[derive(Debug, Clone)]
pub enum Access {
  ReadOnly,
  ReadWrite,
  None,
}

#[derive(Debug, Clone)]
pub enum IP {
  Any,
  Unspecified,
  This(String),
}

#[derive(Debug, Clone)]
pub struct Scope {
  mainaccount: bool,
  connection: bool,
  session: String,
  account: Access,
  trade: Access,
  wallet: Access,
  expires_in: Option<std::time::Duration>,
  ip: IP,
  block_trade: Access,
  block_rfq: Access,
}

impl Scope {
  pub fn dump(&self) -> String {
    let mut parts = vec![];
    if self.mainaccount {
      parts.push("mainaccount".to_string());
    }
    if self.connection {
      parts.push("connection".to_string());
    }
    parts.push(format!("session:{}", self.session));
    match self.account {
      Access::ReadOnly => parts.push("account:read".to_string()),
      Access::ReadWrite => parts.push("account:write".to_string()),
      Access::None => {}
    }
    match self.trade {
      Access::ReadOnly => parts.push("trade:read".to_string()),
      Access::ReadWrite => parts.push("trade:write".to_string()),
      Access::None => {}
    }
    match self.wallet {
      Access::ReadOnly => parts.push("wallet:read".to_string()),
      Access::ReadWrite => parts.push("wallet:write".to_string()),
      Access::None => {}
    }
    if let Some(ref expires_in) = self.expires_in {
      parts.push(format!("expires_in:{}", expires_in.as_secs()));
    }
    match self.ip {
      IP::Any => parts.push("ip:*".to_string()),
      IP::This(ref ip) => parts.push(format!("ip:{}", ip)),
      IP::Unspecified => {}
    }
    match self.block_trade {
      Access::ReadOnly => parts.push("block_trade:read".to_string()),
      Access::ReadWrite => parts.push("block_trade:write".to_string()),
      Access::None => {}
    }
    match self.block_rfq {
      Access::ReadOnly => parts.push("block_rfq:read".to_string()),
      Access::ReadWrite => parts.push("block_rfq:write".to_string()),
      Access::None => {}
    }

    parts.join(",")
  }

  pub fn parse(scope_str: &str) -> Self {
    let mut scope = Scope::default();
    
    for part in scope_str.split(',') {
      match part {
        "mainaccount" => scope.mainaccount = true,
        "connection" => scope.connection = true,
        s if s.starts_with("session:") => scope.session = s[8..].to_string(),
        "account:read" => scope.account = Access::ReadOnly,
        "account:write" => scope.account = Access::ReadWrite,
        "trade:read" => scope.trade = Access::ReadOnly,
        "trade:write" => scope.trade = Access::ReadWrite,
        "wallet:read" => scope.wallet = Access::ReadOnly,
        "wallet:write" => scope.wallet = Access::ReadWrite,
        s if s.starts_with("expires_in:") => {
          if let Ok(secs) = s.split(":").nth(1).unwrap().parse::<u64>() {
            scope.expires_in = Some(std::time::Duration::from_secs(secs));
          }
        }
        "ip:*" => scope.ip = IP::Any,
        s if s.starts_with("ip:") => scope.ip = IP::This(s.split(":").nth(1).unwrap().to_string()),
        "block_trade:read" => scope.block_trade = Access::ReadOnly,
        "block_trade:write" => scope.block_trade = Access::ReadWrite,
        "block_rfq:read" => scope.block_rfq = Access::ReadOnly,
        "block_rfq:write" => scope.block_rfq = Access::ReadWrite,
        _ => {}
      }
    }

    scope
  }

  pub fn default() -> Self {
    Scope {
      mainaccount: false,
      connection: false,
      session: "default".to_string(),
      account: Access::None,
      trade: Access::None,
      wallet: Access::None,
      expires_in: None,
      ip: IP::Unspecified,
      block_trade: Access::None,
      block_rfq: Access::None,
    }
  }

  pub fn named(name: &str) -> Self {
    let mut scope = Scope::default();
    scope.session = name.to_string();
    scope
  }
}

impl<'a> Deserialize<'a> for Scope {
  fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error>
  {
    let scope_str = String::deserialize(deserializer)?;
    Ok(Scope::parse(&scope_str))
  }
}

impl<'a> Serialize for Scope {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&self.dump())
  }
}