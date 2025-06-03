use crate::core::Error;

pub fn parse_json<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T, Error> {
  serde_json::from_value(value)
    .map_err(|e| Error::Json(e))
}