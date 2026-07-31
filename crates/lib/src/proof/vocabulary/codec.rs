use super::{Refusal, refuse};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

pub fn encode(term: &str) -> Result<String, Refusal> {
    canonical(term)?;
    Ok(format!("~{}", URL_SAFE_NO_PAD.encode(term.as_bytes())))
}

pub fn decode(encoded: &str) -> Result<String, Refusal> {
    let payload = encoded
        .strip_prefix('~')
        .ok_or_else(|| refuse("codec", "p64-v1 value has no sigil"))?;
    if payload.is_empty() || payload.contains('=') {
        return Err(refuse("codec", "p64-v1 payload is empty or padded"));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| refuse("codec", format!("cannot decode p64-v1 payload: {error}")))?;
    let term =
        String::from_utf8(bytes).map_err(|_| refuse("codec", "p64-v1 payload is not UTF-8"))?;
    canonical(&term)?;
    if encode(&term)? != encoded {
        return Err(refuse("codec", "p64-v1 payload is not canonical"));
    }
    Ok(term)
}

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn canonical(term: &str) -> Result<(), Refusal> {
    let mut bytes = term.bytes();
    let Some(first) = bytes.next() else {
        return Err(invalid());
    };
    if !first.is_ascii_lowercase() {
        return Err(invalid());
    }
    if !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> Refusal {
    refuse(
        "codec",
        "retired term is not a canonical lowercase ASCII atom",
    )
}
