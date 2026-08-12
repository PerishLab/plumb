use sha2::{Digest, Sha256};

pub fn count(bytes: &[u8]) -> usize {
    bytes.iter().filter(|byte| **byte == b'\n').count()
        + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
}

pub fn flat(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut held = body.iter().peekable();
    while let Some(byte) = held.next() {
        if *byte != b'\r' || held.peek() != Some(&&b'\n') {
            out.push(*byte);
        }
    }
    out
}

pub fn digest(bytes: &[u8]) -> String {
    let mut sponge = Sha256::new();
    sponge.update(bytes);
    sponge
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
