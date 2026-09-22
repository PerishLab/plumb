use super::{OBLIGATIONS, Seat, below, complete, distribution};
use crate::shape::release::Spec;
use clap::CommandFactory as _;
use plumb::land::rejoin::{Stable, latest, tags};
use std::path::Path;
use std::process::Command;

const LISTING: &str = "m\trefs/heads/main\nt1\trefs/tags/v1.0.0\nc1\trefs/tags/v1.0.0^{}\nc2\trefs/tags/v1.1.0\nt3\trefs/tags/v2.0.0-rc.1\nc3\trefs/tags/v2.0.0-rc.1^{}\n";

#[test]
fn paired() {
    let root = crate::Cli::command();
    for obligation in &OBLIGATIONS {
        let mut held = &root;
        for name in obligation.settle {
            held = held
                .find_subcommand(name)
                .unwrap_or_else(|| panic!("{} names no command {name}", obligation.name));
        }
    }
    let stable = Stable {
        marker: "v1.0.0".into(),
        commit: "c".into(),
    };
    assert_eq!(
        OBLIGATIONS[0].hint(&stable),
        "plumb ship dispatch --marker v1.0.0"
    );
    assert_eq!(OBLIGATIONS[1].hint(&stable), "plumb release rejoin");
    assert_eq!(
        OBLIGATIONS[3].hint(&stable),
        "plumb depot consign --version v1.0.0 --kind skill"
    );
}

#[test]
fn distributed() {
    assert_eq!(
        distribution("https://releases.plumb.perish.uk/", "stable", "v1.0.0"),
        "https://releases.plumb.perish.uk/v1/releases/stable/v1.0.0/distribution.json"
    );
    let record = |marker: &str, commit: &str, state: &str| {
        Some(
            format!(r#"{{"marker":"{marker}","commit":"{commit}","state":"{state}"}}"#)
                .into_bytes(),
        )
    };
    assert_eq!(
        complete(record("v1.0.0", "c", "complete"), "v1.0.0", "c"),
        Ok(true)
    );
    assert_eq!(
        complete(record("v1.0.0", "c", "incomplete"), "v1.0.0", "c"),
        Ok(false)
    );
    assert_eq!(
        complete(record("v1.0.0", "d", "complete"), "v1.0.0", "c"),
        Ok(false)
    );
    assert_eq!(
        complete(record("v0.9.0", "c", "complete"), "v1.0.0", "c"),
        Ok(false)
    );
    assert_eq!(complete(None, "v1.0.0", "c"), Ok(false));
    assert!(complete(Some(b"{".to_vec()), "v1.0.0", "c").is_err());
}

#[test]
fn peeled() {
    let stable = latest(tags(LISTING)).expect("stable");
    assert_eq!(
        (stable.marker.as_str(), stable.commit.as_str()),
        ("v1.1.0", "c2")
    );
}

#[test]
fn ceiling() {
    let named = |marker| below(LISTING, marker).map(|held| held.marker);
    assert_eq!(named("v2.0.0-rc.2").as_deref(), Some("v1.1.0"));
    assert_eq!(named("v1.1.0").as_deref(), Some("v1.0.0"));
    assert_eq!(named("v1.0.1-rc.1").as_deref(), Some("v1.0.0"));
    assert_eq!(named("v1.0.0"), None);
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Plumb",
            "-c",
            "user.email=plumb@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn settlement() {
    let fixture = tempfile::tempdir().expect("fixture");
    let work = fixture.path().join("work");
    std::fs::create_dir(&work).expect("work");
    git(fixture.path(), &["init", "-q", "--bare", "remote.git"]);
    git(&work, &["init", "-q", "-b", "main"]);
    git(
        &work,
        &[
            "remote",
            "add",
            "hub",
            fixture.path().join("remote.git").to_str().expect("utf8"),
        ],
    );
    git(&work, &["commit", "-q", "--allow-empty", "-m", "base"]);
    git(&work, &["checkout", "-q", "-b", "release/v1.0.0"]);
    git(&work, &["commit", "-q", "--allow-empty", "-m", "line"]);
    git(&work, &["tag", "-a", "v1.0.0", "-m", "v1.0.0"]);
    git(
        &work,
        &["push", "-q", "hub", "main", "release/v1.0.0", "v1.0.0"],
    );
    let clone = fixture.path().join("clone");
    git(fixture.path(), &["clone", "-q", "remote.git", "clone"]);
    let spec = Spec::decode(
        &clone,
        "[release]\nproduct = \"demo\"\nauthority = \"https://releases.demo.example\"\nbinaries = [\"demo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
        "fixture",
    )
    .expect("spec");
    let owed = |listing: &str| {
        let seat = Seat {
            root: &clone,
            remote: "origin",
            listing,
            spec: &spec,
        };
        seat.rejoined(&below(listing, "v1.1.0").expect("stable"))
    };
    let listing = git(&clone, &["ls-remote", "--heads", "--tags", "origin"]);
    assert!(!owed(&listing).expect("readable"));
    git(&work, &["checkout", "-q", "main"]);
    git(
        &work,
        &["merge", "-q", "--no-ff", "release/v1.0.0", "-m", "settle"],
    );
    git(&work, &["push", "-q", "hub", "main"]);
    let listing = git(&clone, &["ls-remote", "--heads", "--tags", "origin"]);
    assert!(owed(&listing).expect("readable"));
}

#[test]
fn closing() {
    let spec = Spec::decode(
        Path::new("."),
        "[release]\nproduct = \"demo\"\nauthority = \"https://releases.demo.example\"\nbinaries = [\"demo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
        "fixture",
    )
    .expect("spec");
    let stable = Stable {
        marker: "v1.1.0".into(),
        commit: "c2".into(),
    };
    let seat = |listing| Seat {
        root: Path::new("."),
        remote: "origin",
        listing,
        spec: &spec,
    };
    assert!(seat(LISTING).closed(&stable).expect("read"));
    let open = format!("{LISTING}x\trefs/heads/release/v1.1.0\n");
    assert!(!seat(&open).closed(&stable).expect("read"));
}
