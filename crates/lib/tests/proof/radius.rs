use plumb::radius::{Request, Root, check};
use std::path::{Path, PathBuf};

struct Domain {
    fixture: tempfile::TempDir,
}

impl Domain {
    fn new() -> Self {
        Self {
            fixture: tempfile::tempdir().expect("fixture"),
        }
    }

    fn root(&self) -> &Path {
        self.fixture.path()
    }

    fn cargo(&self, name: &str, product: &str, version: &str) -> PathBuf {
        let seat = self.root().join(name);
        std::fs::create_dir_all(&seat).expect("seat");
        std::fs::write(
            seat.join("Cargo.lock"),
            format!(
                "version = 4\n\n[[package]]\nname = \"other\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"{product}\"\nversion = \"{version}\"\n"
            ),
        )
        .expect("lock");
        seat
    }

    fn deno(&self, name: &str, product: &str, version: &str) -> PathBuf {
        let seat = self.root().join(name);
        std::fs::create_dir_all(&seat).expect("seat");
        std::fs::write(
            seat.join("deno.lock"),
            format!("{{\"version\":\"5\",\"specifiers\":{{\"jsr:{product}@^{version}\":\"{version}\"}}}}"),
        )
        .expect("lock");
        seat
    }

    fn bare(&self, name: &str) -> PathBuf {
        let seat = self.root().join(name);
        std::fs::create_dir_all(&seat).expect("seat");
        seat
    }
}

fn named(roots: &[PathBuf]) -> Vec<(String, PathBuf)> {
    roots
        .iter()
        .map(|held| {
            (
                held.file_name()
                    .map(|seen| seen.to_string_lossy().to_string())
                    .unwrap_or_default(),
                held.clone(),
            )
        })
        .collect()
}

fn ask<'a>(roots: &'a [(String, PathBuf)], product: &'a str, candidate: &'a str) -> Request<'a> {
    Request {
        roots: Box::leak(
            roots
                .iter()
                .map(|(label, path)| Root {
                    label,
                    path: path.as_path(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        product,
        candidate,
    }
}

#[test]
fn behind() {
    let domain = Domain::new();
    let roots = vec![
        domain.cargo("alpha", "plumb", "0.18.9"),
        domain.cargo("beta", "plumb", "0.18.11"),
        domain.cargo("gamma", "plumb", "0.18.5"),
    ];
    let report = check(ask(&named(&roots), "plumb", "v0.18.11")).expect("readable domain");
    assert_eq!(report.candidate, "0.18.11");
    assert_eq!(report.seats.len(), 3);
    assert_eq!(report.behind, 2);
    assert!(report.blind.is_empty());
}

#[test]
fn current() {
    let domain = Domain::new();
    let roots = vec![domain.cargo("alpha", "plumb", "0.18.11")];
    let report = check(ask(&named(&roots), "plumb", "0.18.11")).expect("readable domain");
    assert_eq!(report.behind, 0);
    assert!(!report.seats[0].behind);
}

#[test]
fn absent() {
    let domain = Domain::new();
    let roots = vec![domain.cargo("alpha", "locus", "0.2.1")];
    let report = check(ask(&named(&roots), "plumb", "0.18.11")).expect("readable domain");
    assert!(report.seats.is_empty());
    assert_eq!(report.behind, 0);
    assert!(report.blind.is_empty());
}

#[test]
fn bare() {
    let domain = Domain::new();
    let roots = vec![domain.bare("alpha")];
    let report = check(ask(&named(&roots), "plumb", "0.18.11")).expect("readable domain");
    assert!(report.seats.is_empty());
    assert!(report.blind.is_empty());
}

#[test]
fn jsr() {
    let domain = Domain::new();
    let roots = vec![domain.deno("alpha", "@perish/shield", "0.1.0")];
    let report = check(ask(&named(&roots), "@perish/shield", "0.1.1")).expect("readable domain");
    assert_eq!(report.seats.len(), 1);
    assert_eq!(report.seats[0].ecosystem, "jsr");
    assert!(report.seats[0].behind);
}

#[test]
fn unreadable() {
    let domain = Domain::new();
    let roots = vec![
        domain.cargo("alpha", "plumb", "0.18.9"),
        domain.root().join("missing"),
    ];
    let report =
        check(ask(&named(&roots), "plumb", "0.18.11")).expect("a blind root is not a refusal");
    assert_eq!(report.seats.len(), 1);
    assert_eq!(report.blind.len(), 1);
    assert!(report.blind[0].reason.contains("cannot be resolved"));
}

#[test]
fn candidate() {
    let domain = Domain::new();
    let roots = vec![domain.cargo("alpha", "plumb", "0.18.9")];
    let refusal =
        check(ask(&named(&roots), "plumb", "not-a-version")).expect_err("invalid candidate");
    assert_eq!(refusal.kind, "candidate");
    let refusal = check(ask(&named(&roots), "  ", "0.18.11")).expect_err("empty product");
    assert_eq!(refusal.kind, "product");
}

#[test]
fn empty() {
    let report = check(ask(&[], "plumb", "0.18.11")).expect("an empty set is readable");
    assert_eq!(report.behind, 0);
    assert!(report.seats.is_empty());
    assert!(report.blind.is_empty());
}

#[test]
fn garbled() {
    let domain = Domain::new();
    let seat = domain.bare("alpha");
    std::fs::write(
        seat.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"plumb\"\nversion = \"not-a-version\"\n",
    )
    .expect("lock");
    let roots = vec![seat];
    let report = check(ask(&named(&roots), "plumb", "0.18.11")).expect("a garbled lock is blind");
    assert!(report.seats.is_empty());
    assert_eq!(report.blind.len(), 1);
    assert!(report.blind[0].reason.contains("not a version"));
}

#[test]
fn wrapper() {
    let domain = Domain::new();
    let seat = domain.bare("alpha");
    std::fs::create_dir_all(seat.join(".runseal")).expect("seat");
    std::fs::write(
        seat.join(".runseal/deno.lock"),
        "{\"version\":\"5\",\"specifiers\":{\"jsr:@perish/sealkit@*\":\"0.3.4\"}}",
    )
    .expect("lock");
    let roots = vec![seat];
    let report = check(ask(&named(&roots), "@perish/sealkit", "0.4.0")).expect("readable domain");
    assert_eq!(report.seats.len(), 1);
    assert_eq!(report.seats[0].lock, ".runseal/deno.lock");
    assert!(report.seats[0].behind);
}

#[test]
fn labelled() {
    let domain = Domain::new();
    let seat = domain.cargo("alpha", "plumb", "0.18.9");
    let roots = vec![("upstream".to_string(), seat)];
    let report = check(ask(&roots, "plumb", "0.18.11")).expect("readable domain");
    assert_eq!(report.seats[0].label, "upstream");
    let encoded = serde_json::to_string(&report).expect("encodes");
    assert!(
        !encoded.contains("/tmp") && !encoded.contains(std::env::temp_dir().to_str().unwrap()),
        "a published record must carry no filesystem path: {encoded}"
    );
}
