use plumb::rig::Rig;
use std::path::Path;

pub fn validate(root: &Path, rig: &Rig) -> Result<(), String> {
    crate::shape::product::resolve(root, &rig.rules.source).map(|_| ())
}
