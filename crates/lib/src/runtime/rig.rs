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
    pub capsule: PathBuf,
    pub url: String,
    pub activated: bool,
    pub registry_token: String,
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

#[derive(Debug, PartialEq, Cascade)]
#[cascade(section)]
pub struct Depot {
    pub source: String,
    pub channel: String,
    pub seat: PathBuf,
    #[cascade(section)]
    pub authority: Authority,
}

impl Default for Depot {
    fn default() -> Self {
        Self {
            source: DEPOT.to_string(),
            channel: "stable".to_string(),
            seat: PathBuf::new(),
            authority: Authority::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Workflow {
    pub force: bool,
    pub seat: String,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Authority {
    pub access: String,
    pub secret: String,
    pub secret_file: PathBuf,
    pub bucket: String,
    pub endpoint: String,
}

impl Authority {
    pub fn load(&mut self) -> Result<(), String> {
        if !self.secret.is_empty() || self.secret_file.as_os_str().is_empty() {
            return Ok(());
        }
        let held = std::fs::read_to_string(&self.secret_file)
            .map_err(|error| format!("cannot read {}: {error}", self.secret_file.display()))?;
        let held = held.trim();
        if held.is_empty() {
            return Err(format!("{} holds no secret", self.secret_file.display()));
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
            depot: Depot::default(),
            workflow: Workflow::default(),
            site: Site::default(),
            guard: Guard::default(),
        }
    }
}
