mod encoding;
mod reading;

use base64::Engine;

fn wire(key: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    for field in [b"ssh-ed25519".as_slice(), key] {
        result.extend((field.len() as u32).to_be_bytes());
        result.extend(field);
    }
    result
}

fn public(blob: &[u8]) -> Vec<u8> {
    format!(
        "ssh-ed25519 {} fixture\n",
        base64::engine::general_purpose::STANDARD.encode(blob)
    )
    .into_bytes()
}
