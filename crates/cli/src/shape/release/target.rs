#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Tar,
    Zip,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tar => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub triple: String,
    pub key: String,
    pub systems: Vec<String>,
    pub archive: String,
    pub format: Format,
}

pub(super) fn resolve(product: &str, triple: &str) -> Result<Target, String> {
    let (key, systems, format, _runner) = match triple {
        "x86_64-unknown-linux-gnu" => (
            "linux-x64",
            &["Linux:x86_64", "Linux:amd64"][..],
            Format::Tar,
            "linux",
        ),
        "x86_64-apple-darwin" => (
            "darwin-x64",
            &["Darwin:x86_64", "Darwin:amd64"][..],
            Format::Tar,
            "macos",
        ),
        "aarch64-apple-darwin" => (
            "darwin-arm64",
            &["Darwin:arm64", "Darwin:aarch64"][..],
            Format::Tar,
            "macos",
        ),
        "x86_64-pc-windows-msvc" => (
            "windows-x64",
            &["Windows:x86_64", "Windows:amd64"][..],
            Format::Zip,
            "windows",
        ),
        _ => return Err(format!("unsupported release target {triple}")),
    };
    Ok(Target {
        triple: triple.into(),
        key: key.into(),
        systems: systems.iter().map(|system| (*system).into()).collect(),
        archive: format!("{product}-{triple}.{}", format.name()),
        format,
    })
}
