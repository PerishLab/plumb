use crate::windtun::qualification::{Fault, Read};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Key {
    Bytes(String, usize),
    Optional(String, usize),
    Image(String),
}

#[derive(Clone, Eq, PartialEq)]
pub(super) enum Value {
    Bytes(Vec<u8>),
    Optional(Option<Vec<u8>>),
    Image(String),
}

impl Key {
    pub fn moved(&self) -> Fault {
        let (method, address) = match self {
            Self::Bytes(address, _) => ("bytes", address),
            Self::Optional(address, _) => ("optional", address),
            Self::Image(address) => ("image", address),
        };
        Fault::refused(format!(
            "semantic observation moved before final confirmation [{method}; limit:{}; address sha256:{:x}]",
            self.limit(),
            Sha256::digest(address.as_bytes())
        ))
    }

    pub fn read(&self, reader: &dyn Read) -> Result<Value, Fault> {
        match self {
            Self::Bytes(url, limit) => reader.bytes(url, *limit).map(Value::Bytes),
            Self::Optional(url, limit) => reader.optional(url, *limit).map(Value::Optional),
            Self::Image(reference) => reader.image(reference).map(Value::Image),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Bytes(url, _) | Self::Optional(url, _) | Self::Image(url) => url.len(),
        }
    }

    pub fn limit(&self) -> usize {
        match self {
            Self::Bytes(_, limit) | Self::Optional(_, limit) => *limit,
            Self::Image(_) => 4096,
        }
    }
}

impl Value {
    pub fn size(&self) -> usize {
        match self {
            Self::Bytes(raw) | Self::Optional(Some(raw)) => raw.len(),
            Self::Optional(None) => 0,
            Self::Image(value) => value.len(),
        }
    }
}
