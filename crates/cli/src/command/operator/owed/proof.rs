use super::{OBLIGATIONS, Seat, below, observed, said, shipped};
use crate::shape::release::Spec;
use clap::CommandFactory as _;
use plumb::land::rejoin::{Stable, latest, tags};
use plumb::seat::release::Distribution;
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
        "plumb depot consign --version v1.0.0 --kind skill --dir <brief>"
    );
}

#[test]
fn distributed() {
    let record = |commit: &str, state: &str| {
        Distribution::parse(
            format!(r#"{{"marker":"v1.0.0","commit":"{commit}","state":"{state}"}}"#).as_bytes(),
            "v1.0.0",
        )
        .expect("record")
    };
    assert!(shipped(Some(&record("c", "complete")), "c"));
    assert!(!shipped(Some(&record("c", "incomplete")), "c"));
    assert!(!shipped(Some(&record("d", "complete")), "c"));
    assert!(!shipped(None, "c"));
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
    std::fs::write(work.join("fix.txt"), "fixed\n").expect("fix");
    git(&work, &["add", "fix.txt"]);
    git(&work, &["commit", "-q", "-m", "line"]);
    git(&work, &["tag", "-a", "v1.0.0", "-m", "v1.0.0"]);
    git(
        &work,
        &["push", "-q", "hub", "main", "release/v1.0.0", "v1.0.0"],
    );
    let clone = fixture.path().join("clone");
    git(fixture.path(), &["clone", "-q", "remote.git", "clone"]);
    let spec = Spec::decode(
        &clone,
        "[identity]\nns = \"demo\"\n[release]\nauthority = \"https://releases.demo.example\"\nbinaries = [\"demo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
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
    std::fs::write(work.join("fix.txt"), "fixed\n").expect("fix");
    git(&work, &["add", "fix.txt"]);
    git(
        &work,
        &["commit", "-q", "-m", "Squash the line's fix into main"],
    );
    git(&work, &["push", "-q", "hub", "main"]);
    let listing = git(&clone, &["ls-remote", "--heads", "--tags", "origin"]);
    assert!(owed(&listing).expect("readable"));
}

#[test]
fn closing() {
    let spec = Spec::decode(
        Path::new("."),
        "[identity]\nns = \"demo\"\n[release]\nauthority = \"https://releases.demo.example\"\nbinaries = [\"demo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
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

fn skilled(skill: bool) -> Spec {
    let manifest = format!(
        "[identity]\nns = \"demo\"\n[release]\nauthority = \"https://releases.demo.example\"\nbinaries = [\"demo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\nskill = {skill}\n"
    );
    Spec::decode(Path::new("."), &manifest, "fixture").expect("spec")
}

fn brief(home: &Path, version: &str) {
    let path = home.join(".grok/skills/demo");
    std::fs::create_dir_all(&path).expect("seat");
    std::fs::write(
        path.join("metadata.json"),
        format!(r#"{{"schema":1,"name":"demo","version":"{version}","keeper":"demo"}}"#),
    )
    .expect("mark");
}

#[test]
fn observation() {
    let spec = skilled(true);
    let stable = latest(tags(LISTING)).expect("stable");
    let statuses = OBLIGATIONS
        .iter()
        .filter(|held| (held.applies)(&spec))
        .enumerate()
        .map(|(index, obligation)| (obligation, index % 2 == 0))
        .collect::<Vec<_>>();
    let home = tempfile::tempdir().expect("home");
    brief(home.path(), "v1.0.0");
    let seen = observed(&spec, &stable.marker, Some(home.path()));
    let plain = said(&stable, &statuses, None);
    let report = said(&stable, &statuses, seen);
    assert_eq!(statuses.len(), OBLIGATIONS.len());
    assert!(
        plain.contains("owed     its skill lodged on Depot"),
        "{plain}"
    );
    assert!(
        plain.contains("settled  its release line closed"),
        "{plain}"
    );
    let observed = report
        .strip_prefix(&format!("{plain}\n"))
        .unwrap_or_else(|| panic!("obligations changed: {report}"));
    assert_eq!(
        observed,
        format!(
            "observed on this machine, not owed:\n  grok seat {} holds the v1.0.0 brief, below v1.1.0: install demo v1.1.0, then run demo skill upgrade",
            home.path().join(".grok/skills/demo").display()
        )
    );
}

#[test]
fn unobserved() {
    let home = tempfile::tempdir().expect("home");
    assert_eq!(observed(&skilled(true), "v1.1.0", Some(home.path())), None);
    assert_eq!(observed(&skilled(true), "v1.1.0", None), None);
    brief(home.path(), "v1.1.0");
    assert_eq!(observed(&skilled(true), "v1.1.0", Some(home.path())), None);
    brief(home.path(), "v1.0.0");
    assert_eq!(observed(&skilled(false), "v1.1.0", Some(home.path())), None);
    assert!(observed(&skilled(true), "v1.1.0", Some(home.path())).is_some());
}
