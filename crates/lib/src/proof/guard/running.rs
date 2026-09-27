use super::{Authority, Expected, Verified};
use std::path::Path;

pub(crate) fn verified(root: &Path, commit: &str) -> Result<Verified, String> {
    let proof = super::commit(root, commit)?;
    let expected = Expected::held(&proof);
    Authority::running()?.judge(root, proof, &expected)
}
