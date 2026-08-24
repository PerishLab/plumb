#[path = "workflow/lane.rs"]
mod lane;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct Seat(PathBuf);

impl Seat {
    pub fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .output()
            .expect("git");
        assert!(out.status.success(), "git {args:?}");
    }

    pub fn declared(&self, body: &str) {
        fs::write(self.0.join("plumb.toml"), body).expect("manifest");
        self.git(&["add", "-A"]);
    }

    pub fn shown(&self) -> (String, bool) {
        let out = plumb(&["workflow", "status", self.0.to_str().expect("path")])
            .output()
            .expect("run");
        (
            String::from_utf8_lossy(&out.stdout).to_string(),
            out.status.success(),
        )
    }

    pub fn verb(&self, deed: &str, key: &str, forced: bool) -> (String, bool) {
        let mut held = plumb(&["workflow", deed, key, self.0.to_str().expect("path")]);
        if forced {
            held.env("PLUMB_WORKFLOW_FORCE", "true");
        }
        let out = held.output().expect("run");
        (
            String::from_utf8_lossy(&out.stdout).to_string(),
            out.status.success(),
        )
    }

    pub fn lane(&self) -> String {
        let out = plumb(&["lane", self.0.to_str().expect("path")])
            .output()
            .expect("run");
        let _ = out;
        fs::read_to_string(self.0.join(".forgejo/workflows/guard.yml")).unwrap_or_default()
    }

    pub fn rendered(&self) -> String {
        plumb(&["lane", self.0.to_str().expect("path"), "--write"])
            .output()
            .expect("run");
        self.lane()
    }

    pub fn wrote(&self, path: &str, body: &str) {
        fs::write(self.0.join(path), body).expect("leaf");
        self.git(&["add", "-A"]);
    }
}

pub fn seat(name: &str) -> Seat {
    let path = std::env::temp_dir().join(format!("plumb-workflow-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("crates/cli/src")).expect("seat");
    fs::create_dir_all(path.join("apps/web/src")).expect("seat");
    fs::write(path.join("crates/cli/src/main.rs"), "fn main() {}\n").expect("leaf");
    fs::write(path.join("apps/web/src/app.ts"), "export {};\n").expect("leaf");
    fs::write(path.join("Cargo.toml"), "[workspace]\n").expect("leaf");
    let seat = Seat(path);
    seat.git(&["init", "--initial-branch", "main"]);
    seat.git(&["config", "user.email", "seat@example.com"]);
    seat.git(&["config", "user.name", "seat"]);
    seat
}

fn plumb(args: &[&str]) -> Command {
    let mut held = Command::new(env!("CARGO_BIN_EXE_plumb"));
    held.args(args);
    for name in [
        "PLUMB_LOCK_ACCESS",
        "PLUMB_LOCK_SECRET",
        "PLUMB_LOCK_BUCKET",
        "PLUMB_LOCK_ENDPOINT",
        "PLUMB_WORKFLOW_FORCE",
        "PLUMB_WORKFLOW_SEAT",
    ] {
        held.env_remove(name);
    }
    held
}

fn digest(text: &str, key: &str) -> String {
    text.lines()
        .find(|line| line.trim_start().starts_with(key))
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("digest")
        .to_string()
}

const PAIR: &str = "[workflow.hash.guard]\n\
\"rust\" = [\"crates\", \"Cargo.toml\"]\n\
\"web\" = [\"apps\"]\n";

#[test]
fn reads() {
    let root = seat("reads");
    root.declared(PAIR);
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    assert!(text.contains("guard/rust"), "{text}");
    assert!(text.contains("guard/web"), "{text}");
}

#[test]
fn expands() {
    let root = seat("expands");
    root.declared(&format!(
        "{PAIR}\"proofs\" = [\"key://guard/rust\", \"key://guard/web\"]\n"
    ));
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("guard/proofs"))
        .expect("proofs");
    assert!(line.contains("3 paths"), "{line}");
}

#[test]
fn refuses() {
    let root = seat("refuses");
    root.declared(&format!("{PAIR}\"proofs\" = [\"key://guard/gone\"]\n"));
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("names no key called guard/gone"), "{text}");
}

#[test]
fn cycles() {
    let root = seat("cycles");
    root.declared(
        "[workflow.hash.guard]\n\
         \"rust\" = [\"key://guard/web\"]\n\
         \"web\" = [\"key://guard/rust\"]\n",
    );
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("key cycle"), "{text}");
}

#[test]
fn lanes() {
    let root = seat("lanes");
    root.declared("[workflow.hash.nowhere]\n\"rust\" = [\"crates\"]\n");
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("names no lane called nowhere"), "{text}");
}

#[test]
fn suites() {
    let root = seat("suites");
    root.declared("[workflow.hash.guard]\n\"rust\" = [\"suite://cargo\"]\n");
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    assert!(text.contains("suite://cargo"), "{text}");
    assert!(text.contains("Cargo.toml"), "{text}");
}

#[test]
fn unknown() {
    let root = seat("unknown");
    root.declared("[workflow.hash.guard]\n\"rust\" = [\"suite://gone\"]\n");
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("names no suite called gone"), "{text}");
}

#[test]
fn namespaces() {
    let root = seat("namespaces");
    root.declared("[workflow.hash.ship.cargo]\n\"rehearse\" = [\"suite://cargo\"]\n");
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    assert!(text.contains("ship/cargo.rehearse"), "{text}");
}

#[test]
fn moves() {
    let root = seat("moves");
    root.declared(PAIR);
    let (first, _) = root.shown();
    root.wrote("apps/web/src/app.ts", "export const held = 1;\n");
    let (second, _) = root.shown();
    assert_eq!(digest(&first, "guard/rust"), digest(&second, "guard/rust"));
    assert_ne!(digest(&first, "guard/web"), digest(&second, "guard/web"));
}

#[test]
fn declarations() {
    let root = seat("declarations");
    root.declared(PAIR);
    let (first, _) = root.shown();
    root.declared(
        "[workflow.hash.guard]\n\
         \"rust\" = [\"crates\", \"Cargo.toml\", \"Cargo.lock\"]\n\
         \"web\" = [\"apps\"]\n",
    );
    let (second, _) = root.shown();
    assert_ne!(digest(&first, "guard/rust"), digest(&second, "guard/rust"));
}

#[test]
fn loose() {
    let root = seat("loose");
    root.declared(PAIR);
    let (text, ok) = root.verb("hash", "guard/rust", false);
    assert!(!ok, "{text}");
    assert!(text.contains("runs"), "{text}");
}

#[test]
fn forced() {
    let root = seat("forced");
    root.declared(PAIR);
    let (text, ok) = root.verb("hash", "guard/rust", true);
    assert!(!ok, "{text}");
    assert!(text.contains("forces every step"), "{text}");
}

#[test]
fn absent() {
    let root = seat("absent");
    root.declared(PAIR);
    let (text, ok) = root.verb("hash", "guard/gone", false);
    assert!(!ok, "{text}");
}

#[test]
fn tolerates() {
    let root = seat("tolerates");
    root.declared(PAIR);
    let (text, ok) = root.verb("lock", "guard/rust", false);
    assert!(ok, "{text}");
}

#[test]
fn seated() {
    let root = seat("seated");
    root.declared(PAIR);
    let (text, ok) = root.verb("lock", "guard/rust", false);
    assert!(ok, "{text}");
    let (text, ok) = root.verb("hash", "guard/rust", false);
    assert!(!ok, "{text}");
    assert!(text.contains("names no lock seat"), "{text}");
}
