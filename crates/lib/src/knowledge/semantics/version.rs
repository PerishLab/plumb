use semver::{BuildMetadata, Prerelease, Version};
use std::collections::BTreeSet;

use super::{Decision, wire::Wire};

pub(super) struct Markers<'a>(pub &'a [String]);

impl Markers<'_> {
    pub fn parse(marker: &str) -> Result<Version, String> {
        let raw = marker
            .strip_prefix('v')
            .ok_or("release markers require v")?;
        let version = Version::parse(raw).map_err(|error| format!("invalid marker: {error}"))?;
        if !version.build.is_empty() || marker != format!("v{version}") {
            return Err("release markers require canonical versions without build metadata".into());
        }
        if !version.pre.is_empty() {
            let (channel, turn) = version
                .pre
                .as_str()
                .split_once('.')
                .ok_or("invalid channel")?;
            if !["alpha", "beta", "rc"].contains(&channel) {
                return Err("release prereleases require alpha, beta or rc".into());
            }
            if !Wire::turn(turn) {
                return Err(
                    "release prereleases require alpha, beta or rc and a positive turn".into(),
                );
            }
        }
        Ok(version)
    }

    pub fn stable(marker: &str) -> Result<Version, String> {
        let version = Self::parse(marker)?;
        if !version.pre.is_empty() {
            return Err("baseline and anchor require stable markers".into());
        }
        Ok(version)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.0.len() > 4096 {
            return Err("too many reserved release markers".into());
        }
        let mut names = BTreeSet::new();
        for marker in self.0 {
            Self::parse(marker)?;
            if !names.insert(marker) {
                return Err("reserved release markers repeat".into());
            }
        }
        Ok(())
    }

    pub fn highest(&self, baseline: &str) -> Result<Version, String> {
        let stable = Self::stable(baseline)?;
        let mut highest = stable.clone();
        for marker in self.0 {
            let version = Self::parse(marker)?;
            if version.pre.is_empty() && version > stable {
                return Err("a newer stable marker invalidates the supplied baseline".into());
            }
            highest = highest.max(version);
        }
        Ok(highest)
    }

    pub fn next(&self, baseline: &str, decision: Decision) -> Result<Option<String>, String> {
        let stable = Self::stable(baseline)?;
        let highest = self.highest(baseline)?;
        let mut next = stable.clone();
        match decision {
            Decision::Same => {
                next.patch = Self::increment(stable.patch)?.max(highest.patch);
                if (highest.major, highest.minor) != (stable.major, stable.minor) {
                    return Ok(None);
                }
            }
            Decision::Changed => {
                if highest.major != stable.major {
                    return Ok(None);
                }
                next.minor = Self::increment(stable.minor)?.max(highest.minor);
                next.patch = 0;
                if next.minor == highest.minor && highest.patch > 0 {
                    next.minor = Self::increment(next.minor)?;
                }
            }
        }
        next.pre = Prerelease::EMPTY;
        next.build = BuildMetadata::EMPTY;
        if next <= highest {
            return Ok(None);
        }
        Ok(Some(format!("v{next}")))
    }

    fn increment(value: u64) -> Result<u64, String> {
        value
            .checked_add(1)
            .ok_or_else(|| "release version is exhausted".into())
    }
}
