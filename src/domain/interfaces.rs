use serde::{de::DeserializeOwned, ser::Serialize};

#[allow(dead_code)]
pub trait Event: DeserializeOwned + Serialize + Unpin + Send + Sync + 'static {}

#[allow(dead_code)]
pub trait Command: DeserializeOwned {}

#[allow(dead_code)]
pub trait Model: Serialize + DeserializeOwned + Unpin + Send + Sync + 'static {}
