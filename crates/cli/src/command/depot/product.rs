use plumb::rig::Rig;
use std::path::Path;

pub use crate::shape::product::Target;

pub fn resolve(
    root: &Path,
    rig: &Rig,
    derivative: plumb::depot::v3::Kind,
) -> Result<Target, String> {
    let target = crate::shape::product::resolve(root, &rig.rules.source)?;
    target.require(derivative)?;
    Ok(target)
}
