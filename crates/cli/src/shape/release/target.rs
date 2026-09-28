#[derive(Clone, Debug)]
pub struct Target {
    pub triple: String,
}

pub(super) fn resolve(triple: &str) -> Result<Target, String> {
    match triple {
        "x86_64-unknown-linux-gnu"
        | "x86_64-apple-darwin"
        | "aarch64-apple-darwin"
        | "x86_64-pc-windows-msvc" => Ok(Target {
            triple: triple.into(),
        }),
        _ => Err(format!("unsupported release target {triple}")),
    }
}
