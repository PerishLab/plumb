use crate::command::release::ReleaseMarker;
use plumb::rule::Production;
use sha2::{Digest, Sha256};

pub(in crate::command::ship) fn contract(
    marker: &ReleaseMarker,
    triple: &str,
) -> Result<Production, String> {
    let mut contract = super::super::transport::production::contract(marker, triple)?;
    let mut digest = Sha256::new();
    for source in [
        include_str!("proof.rs"),
        include_str!("mod.rs"),
        include_str!("archive.rs"),
        include_str!("sign.rs"),
        include_str!("../../../../../lib/src/runtime/identity/codec.rs"),
        include_str!("../../../../../lib/src/runtime/identity/image.rs"),
    ] {
        let source = source.replace("\r\n", "\n");
        digest.update((source.len() as u64).to_le_bytes());
        digest.update(source.as_bytes());
    }
    contract.implementation = format!("{:x}", digest.finalize());
    contract.digest()?;
    Ok(contract)
}
