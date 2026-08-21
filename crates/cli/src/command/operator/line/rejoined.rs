use semver::Version;
use std::path::Path;
use std::process::{Command, Output};

pub struct Seat<'a>(pub &'a Path);

impl Seat<'_> {
    pub fn rejoined(&self, version: &str) -> Result<(), String> {
        let base = self.reference("origin/main")?;
        let Some((name, point)) = self.last(version)? else {
            return Ok(());
        };
        if super::super::topology::ancestor(self.0, &point, &base)? {
            return Ok(());
        }
        Err(format!(
            "stable {name} stands at {point}, which origin/main does not hold; \
             run plumb release rejoin --version {name} before opening the next line"
        ))
    }

    fn last(&self, version: &str) -> Result<Option<(String, String)>, String> {
        let listed = read("list release points", self.git(["tag", "--list", "v*"]))?;
        let held = listed
            .lines()
            .map(str::trim)
            .filter(|name| *name != version)
            .filter_map(|name| {
                let held = Version::parse(name.trim_start_matches('v')).ok()?;
                held.pre.is_empty().then_some((held, name.to_string()))
            })
            .max_by(|(left, _), (right, _)| left.cmp(right));
        let Some((_, name)) = held else {
            return Ok(None);
        };
        let point = self.reference(&format!("{name}^{{commit}}"))?;
        Ok(Some((name, point)))
    }

    fn reference(&self, name: &str) -> Result<String, String> {
        read(
            &format!("resolve {name}"),
            self.git(["rev-parse", "--verify", name]),
        )
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Result<Output, String> {
        Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}

fn read(action: &str, output: Result<Output, String>) -> Result<String, String> {
    let output = output?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
