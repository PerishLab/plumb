use base64::Engine;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use sha2::{Digest, Sha256};

pub(super) fn identity(public: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(public).map_err(|_| "host public key is not UTF-8")?;
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let line = lines.next().ok_or("host public key is empty")?;
    if lines.next().is_some() {
        return Err("host public key must contain exactly one record".into());
    }
    let blob = decode(line)?;
    validate(&blob)?;
    Ok(format!(
        "ssh-ed25519:SHA256:{}",
        STANDARD_NO_PAD.encode(Sha256::digest(&blob))
    ))
}

fn decode(line: &str) -> Result<Vec<u8>, String> {
    let mut fields = line.split_whitespace();
    if fields.next() != Some("ssh-ed25519") {
        return Err("host public key must declare ssh-ed25519".into());
    }
    let encoded = fields.next().ok_or("host public key has no encoded blob")?;
    let blob = STANDARD
        .decode(encoded)
        .map_err(|_| "host public key has invalid Base64")?;
    if STANDARD.encode(&blob) != encoded {
        return Err("host public key Base64 is not canonical".into());
    }
    Ok(blob)
}

fn validate(blob: &[u8]) -> Result<(), String> {
    let (algorithm, remaining) = field(blob)?;
    if algorithm != b"ssh-ed25519" {
        return Err("host public key wire algorithm does not match ssh-ed25519".into());
    }
    let (key, trailing) = field(remaining)?;
    if key.len() != 32 || !trailing.is_empty() {
        return Err("host public key must carry 32 key bytes and no trailing wire data".into());
    }
    Ok(())
}

fn field(bytes: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let header: [u8; 4] = bytes
        .get(..4)
        .ok_or("host public key wire length is truncated")?
        .try_into()
        .expect("checked four-byte header");
    let size = u32::from_be_bytes(header) as usize;
    let body = &bytes[4..];
    if size > body.len() {
        return Err("host public key wire field is truncated".into());
    }
    Ok(body.split_at(size))
}
