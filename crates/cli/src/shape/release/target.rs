#[derive(Clone, Debug)]
pub struct Target {
    pub triple: String,
}

pub(super) const TARGETS: [&str; 3] = [
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
];

pub(super) fn resolve(triple: &str) -> Result<Target, String> {
    if TARGETS.contains(&triple) {
        Ok(Target {
            triple: triple.into(),
        })
    } else {
        Err(format!(
            "unsupported release target {triple}; wharf releases {}",
            TARGETS.join(", ")
        ))
    }
}
