use plumb::rig::Rig;
use std::path::Path;

pub fn require(root: &Path, rig: &Rig, derivative: plumb::depot::v3::Kind) -> Result<(), String> {
    crate::shape::product::resolve(root, &rig.rules.source)?.require(derivative)
}
