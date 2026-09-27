#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Tar,
    Zip,
}

#[derive(Clone, Debug)]
pub struct Target {
    pub triple: String,
    pub format: Format,
}

pub(super) fn resolve(triple: &str) -> Result<Target, String> {
    let format = match triple {
        "x86_64-unknown-linux-gnu" | "x86_64-apple-darwin" | "aarch64-apple-darwin" => Format::Tar,
        "x86_64-pc-windows-msvc" => Format::Zip,
        _ => return Err(format!("unsupported release target {triple}")),
    };
    Ok(Target {
        triple: triple.into(),
        format,
    })
}
