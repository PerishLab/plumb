use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use sha2::Sha256;

pub struct Codec;

impl Codec {
    pub fn bytes(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }
    pub fn sha(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }
    pub fn digest(value: &Value) -> String {
        Self::sha(&Self::bytes(value))
    }

    pub fn content(path: &str, raw: &[u8]) -> Value {
        let mut hash = Sha1::new();
        hash.update(format!("blob {}\0", raw.len()).as_bytes());
        hash.update(raw);
        json!({"type":"file", "path":path, "size":raw.len(), "encoding":"base64", "sha":format!("{:x}",hash.finalize()), "content":STANDARD.encode(raw)})
    }

    pub fn reference(url: &str, raw: &[u8]) -> Value {
        json!({"url":url, "size":raw.len(), "sha256":Self::sha(raw)})
    }
}
