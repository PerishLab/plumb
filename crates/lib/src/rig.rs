use crate::config::Cascade;
use std::path::PathBuf;

const RELEASES: &str = "https://releases.plumb.perish.uk";

#[derive(Debug, PartialEq, Cascade)]
pub struct Rig {
    pub home: String,
    pub releases: String,
    #[cascade(section)]
    pub locus: Locus,
    #[cascade(section)]
    pub release: Release,
    #[cascade(section)]
    pub publish: Authority,
    #[cascade(section)]
    pub activate: Authority,
    #[cascade(section)]
    pub site: Site,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Locus {
    pub enabled: bool,
    #[cascade(section)]
    pub report: Report,
    #[cascade(section)]
    pub trace: Trace,
    #[cascade(section)]
    pub target: Target,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Report {
    pub file: PathBuf,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Trace {
    pub file: PathBuf,
    pub id: String,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Target {
    pub collectors: String,
}

#[derive(Debug, PartialEq, Cascade)]
pub struct Harness {
    pub run_poll_ms: u64,
    pub run_timeout_ms: u64,
    pub guard_register_ms: u64,
    pub guard_pending_ms: u64,
    pub guard_timeout_ms: u64,
}

#[derive(Debug, PartialEq, Cascade)]
#[cascade(section)]
pub struct Mint {
    pub account: String,
    pub api: String,
    pub token: String,
}

impl Default for Mint {
    fn default() -> Self {
        Self {
            account: String::new(),
            api: "https://api.cloudflare.com/client/v4".to_string(),
            token: String::new(),
        }
    }
}

#[derive(Debug, PartialEq, Cascade)]
#[cascade(section)]
pub struct Site {
    pub account: String,
    pub api: String,
    pub blind: bool,
    pub delay: u64,
    pub domain: String,
    pub token: String,
    pub turns: usize,
}

impl Default for Site {
    fn default() -> Self {
        Self {
            account: String::new(),
            api: "https://api.cloudflare.com/client/v4".to_string(),
            blind: false,
            delay: 5_000,
            domain: String::new(),
            token: String::new(),
            turns: 10,
        }
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self {
            run_poll_ms: 10_000,
            run_timeout_ms: 3_600_000,
            guard_register_ms: 5_000,
            guard_pending_ms: 10_000,
            guard_timeout_ms: 240_000,
        }
    }
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Release {
    pub root: PathBuf,
    pub channel: String,
    pub version: String,
    pub commit: String,
    pub source: String,
    pub base: String,
    pub target: String,
    pub artifacts: PathBuf,
    pub output: PathBuf,
    pub promotion: Option<PathBuf>,
    pub promotion_channel: String,
    pub promotion_version: String,
    pub capsule: PathBuf,
    pub url: String,
    pub activated: bool,
    pub registry_token: String,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Authority {
    pub access: String,
    pub secret: String,
    pub bucket: String,
    pub endpoint: String,
}

impl Default for Rig {
    fn default() -> Self {
        Rig {
            home: crate::config::data("plumb")
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            releases: RELEASES.to_string(),
            locus: Locus::default(),
            release: Release {
                root: PathBuf::from("."),
                base: "origin/main".to_string(),
                ..Release::default()
            },
            publish: Authority::default(),
            activate: Authority::default(),
            site: Site::default(),
        }
    }
}
