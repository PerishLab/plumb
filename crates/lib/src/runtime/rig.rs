use crate::config::Cascade;
use std::path::PathBuf;

const RELEASES: &str = "https://releases.plumb.perish.uk";
const DEPOT: &str = "https://depot.plumb.perish.uk";

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
    pub lock: Authority,
    #[cascade(section)]
    pub rules: Rules,
    #[cascade(section)]
    pub depot: Depot,
    #[cascade(section)]
    pub workflow: Workflow,
    #[cascade(section)]
    pub site: Site,
    #[cascade(section)]
    pub guard: Guard,
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
    #[cascade(section)]
    pub run: Run,
    #[cascade(section)]
    pub guard: Gate,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Run {
    #[cascade(name = "poll_ms")]
    pub poll: Millis,
    #[cascade(name = "timeout_ms")]
    pub timeout: Millis,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Gate {
    #[cascade(name = "register_ms")]
    pub register: Millis,
    #[cascade(name = "pending_ms")]
    pub pending: Millis,
    #[cascade(name = "timeout_ms")]
    pub timeout: Millis,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(transparent)]
pub struct Millis(u64);

impl Millis {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.0)
    }

    pub fn seconds(&self) -> u64 {
        self.0 / 1000
    }
}

impl crate::config::Env for Millis {
    fn read(value: &str) -> Result<Self, String> {
        value
            .parse()
            .map(Self)
            .map_err(|error: std::num::ParseIntError| error.to_string())
    }
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
            run: Run {
                poll: Millis::new(10_000),
                timeout: Millis::new(3_600_000),
            },
            guard: Gate {
                register: Millis::new(5_000),
                pending: Millis::new(10_000),
                timeout: Millis::new(240_000),
            },
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
    pub capsule: PathBuf,
    pub url: String,
    pub activated: bool,
    #[cascade(name = "registry_token")]
    pub credential: String,
    pub toolchain: String,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Guard {
    pub api: String,
    pub repository: String,
    pub token: String,
    pub contexts: String,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Depot {
    #[cascade(section)]
    pub authority: Authority,
}

#[derive(Debug, PartialEq, Cascade)]
#[cascade(section)]
pub struct Rules {
    pub source: String,
    pub channel: String,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            source: DEPOT.to_string(),
            channel: "stable".to_string(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Workflow {
    pub force: bool,
    pub seat: String,
    #[cascade(section)]
    pub inventory: Authority,
}

#[derive(Clone, Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Authority {
    pub access: String,
    pub secret: String,
    #[cascade(name = "secret_file")]
    pub file: PathBuf,
    pub bucket: String,
    pub endpoint: String,
    pub fingerprint: String,
    pub url: String,
}

impl Authority {
    pub fn load(&mut self) -> Result<(), String> {
        if !self.secret.is_empty() || self.file.as_os_str().is_empty() {
            return Ok(());
        }
        let held = std::fs::read_to_string(&self.file)
            .map_err(|error| format!("cannot read {}: {error}", self.file.display()))?;
        let held = held.trim();
        if held.is_empty() {
            return Err(format!("{} holds no secret", self.file.display()));
        }
        self.secret = held.to_string();
        Ok(())
    }
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
            lock: Authority::default(),
            rules: Rules::default(),
            depot: Depot::default(),
            workflow: Workflow::default(),
            site: Site::default(),
            guard: Guard::default(),
        }
    }
}
