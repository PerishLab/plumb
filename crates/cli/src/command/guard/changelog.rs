use semver::Version;
use std::path::Path;

pub(crate) use plumb::changelog::Proof;

#[derive(Debug)]
pub struct Scaffold {
    pub version: String,
    pub units: usize,
    pub budget: usize,
    pub commits: usize,
}

struct Repo<'a>(&'a Path);

pub fn stamped(version: &str) -> String {
    match version.strip_prefix('v') {
        Some(_) => version.to_string(),
        None => format!("v{version}"),
    }
}

pub fn prove(root: &Path, home: &Path, version: &str) -> Result<Proof, String> {
    let repo = Repo(root);
    let version = identity(version)?;
    let stamped = stamped(&version);
    let candidate = repo.point(&stamped)?;
    let previous = repo.prior(&version, &candidate)?;
    plumb::changelog::prove(plumb::changelog::Claim {
        root,
        home,
        version: &version,
        previous: previous.as_deref(),
        candidate: &candidate,
    })
}

pub fn scaffold(root: &Path, home: &Path, version: &str) -> Result<Scaffold, String> {
    let repo = Repo(root);
    let base = identity(version)?;
    let marker = stamped(&base);
    let candidate = repo.point(&marker)?;
    let previous = repo.prior(&base, &candidate)?;
    for tongue in ["en", "zh"] {
        for leaf in ["INDEX.md", "MIGRATION.md"] {
            let path = home.join(tongue).join(leaf);
            let filled = path.exists()
                && !std::fs::read_to_string(&path)
                    .unwrap_or_default()
                    .trim()
                    .is_empty();
            if filled {
                return Err(format!(
                    "refusing to overwrite non-empty changelog file {}",
                    path.display()
                ));
            }
        }
    }
    let subjects = repo.subjects(previous.as_deref(), &candidate)?;
    let items = if subjects.is_empty() {
        "- Summary of changes".to_string()
    } else {
        subjects
            .iter()
            .take(20)
            .map(|line| format!("- {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    for tongue in ["en", "zh"] {
        let dir = home.join(tongue);
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
        std::fs::write(dir.join("INDEX.md"), format!("# {marker}\n\n{items}\n"))
            .map_err(|error| format!("cannot write {tongue}/INDEX.md: {error}"))?;
        std::fs::write(
            dir.join("MIGRATION.md"),
            format!("# {marker} Migration\n\nNo migration steps required.\n"),
        )
        .map_err(|error| format!("cannot write {tongue}/MIGRATION.md: {error}"))?;
    }
    let proof = plumb::changelog::prove(plumb::changelog::Claim {
        root,
        home,
        version: &base,
        previous: previous.as_deref(),
        candidate: &candidate,
    })?;
    let budget = proof
        .languages
        .values()
        .next()
        .map_or(120, |lang| lang.budget);
    Ok(Scaffold {
        version: marker,
        units: proof.units,
        budget,
        commits: subjects.len(),
    })
}

pub fn identity(version: &str) -> Result<String, String> {
    let parsed = Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("invalid version {version}: {error}"))?;
    Ok(format!(
        "{}.{}.{}",
        parsed.major, parsed.minor, parsed.patch
    ))
}

impl Repo<'_> {
    fn point(&self, tag: &str) -> Result<String, String> {
        let output = plumb::config::detached("git")
            .current_dir(self.0)
            .args(["rev-list", "-n", "1", tag])
            .output()
            .map_err(|error| format!("cannot run git rev-list: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{tag} names no point in this repository; a release note describes a version that shipped"
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn prior(&self, version: &str, candidate: &str) -> Result<Option<String>, String> {
        let held = Version::parse(version)
            .map_err(|error| format!("invalid version {version}: {error}"))?;
        let output = plumb::config::detached("git")
            .current_dir(self.0)
            .args(["tag", "--list", "v*"])
            .output()
            .map_err(|error| format!("cannot run git tag: {error}"))?;
        let mut found = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| Version::parse(line.trim().trim_start_matches('v')).ok())
            .filter(|held| held.pre.is_empty())
            .filter(|other| other < &held)
            .collect::<Vec<_>>();
        found.sort();
        while let Some(base) = found.pop() {
            let commit = self.point(&format!("v{base}"))?;
            let status = plumb::config::detached("git")
                .current_dir(self.0)
                .args(["merge-base", "--is-ancestor", &commit, candidate])
                .status()
                .map_err(|error| format!("cannot run git merge-base: {error}"))?;
            if status.success() {
                return Ok(Some(commit));
            }
        }
        Ok(None)
    }

    fn subjects(&self, previous: Option<&str>, candidate: &str) -> Result<Vec<String>, String> {
        let range = match previous {
            Some(base) => format!("{base}..{candidate}"),
            None => candidate.to_string(),
        };
        let output = plumb::config::detached("git")
            .current_dir(self.0)
            .args(["log", "--format=%s", &range])
            .output()
            .map_err(|error| format!("cannot run git log: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cannot list commits for {range}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{prove, scaffold};
    use std::path::Path;
    use std::process::Command;

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "user.name=Plumb",
                "-c",
                "user.email=plumb@example.invalid",
            ])
            .args(args)
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn builds() {
        let repo = tempfile::tempdir().expect("repo");
        let root = repo.path();
        git(root, &["init", "-q"]);
        std::fs::write(root.join("lib.rs"), "pub fn one() {}\n").expect("write");
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "initial"]);
        git(root, &["tag", "-a", "v0.1.0", "-m", "v0.1.0"]);
        std::fs::write(root.join("lib.rs"), "pub fn two() {}\n").expect("write");
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "feat: update api"]);
        git(root, &["tag", "-a", "v0.2.0", "-m", "v0.2.0"]);

        let notes = tempfile::tempdir().expect("notes");
        let built = scaffold(root, notes.path(), "v0.2.0").expect("scaffold");
        assert_eq!(built.version, "v0.2.0");
        assert_eq!(built.commits, 1);
        assert!(built.budget >= 120);
        prove(root, notes.path(), "v0.2.0").expect("scaffolded notes prove");

        let refused = scaffold(root, notes.path(), "v0.2.0").expect_err("no overwrite");
        assert!(refused.contains("refusing to overwrite"), "{refused}");
    }
}
