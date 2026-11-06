use crate::windtun::qualification::{Fault, Read};
use serde::de::DeserializeOwned;
use sha1::{Digest, Sha1};

pub(super) struct Wire;

impl Wire {
    pub const LIMIT: usize = 4 * 1024 * 1024;
    pub const TOTAL: usize = 64 * 1024 * 1024;

    pub fn document<T: DeserializeOwned>(reader: &impl Read, url: &str) -> Result<T, Fault> {
        let raw = reader.bytes(url, Self::LIMIT)?;
        if raw.is_empty() || raw.len() > Self::LIMIT {
            return Err(Fault::refused("public document exceeds bounds"));
        }
        serde_json::from_slice(&raw).map_err(|_| Fault::refused("invalid public document"))
    }

    pub fn require(ok: bool, reason: &str) -> Result<(), Fault> {
        if ok {
            Ok(())
        } else {
            Err(Fault::refused(reason))
        }
    }

    pub fn name(value: &str) -> Result<(), Fault> {
        super::super::wire::Wire::name(value).map_err(Fault::refused)
    }

    pub fn path(value: &str) -> Result<(), Fault> {
        super::super::wire::Wire::path(value).map_err(Fault::refused)
    }

    pub fn hex(value: &str) -> Result<(), Fault> {
        super::super::wire::Wire::hex(value, 40).map_err(Fault::refused)
    }

    pub fn blob(raw: &[u8]) -> String {
        let mut hash = Sha1::new();
        hash.update(format!("blob {}\0", raw.len()).as_bytes());
        hash.update(raw);
        format!("{:x}", hash.finalize())
    }
}
