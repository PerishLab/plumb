use super::{next, promoted, reference, repository};

const LISTING: &str = "aaa\trefs/heads/release/v0.38.0\nbbb\trefs/tags/v0.38.0-beta.1\nccc\trefs/tags/v0.38.0-beta.1^{}\nddd\trefs/tags/v0.38.0-beta.3\neee\trefs/tags/v0.38.1-beta.9\n";

#[test]
fn numbering() {
    assert_eq!(next(LISTING, "v0.38.0", "beta"), 4);
    assert_eq!(next(LISTING, "v0.39.0", "beta"), 1);
    assert_eq!(next(LISTING, "v0.38.0", "rc"), 1);
}

#[test]
fn listings() {
    assert_eq!(
        reference(LISTING, "refs/heads/release/v0.38.0").as_deref(),
        Some("aaa")
    );
    assert_eq!(
        reference(LISTING, "refs/tags/v0.38.0-beta.1").as_deref(),
        Some("bbb")
    );
    assert_eq!(reference(LISTING, "refs/heads/release/v0.37.0"), None);
}

#[test]
fn remotes() {
    assert_eq!(
        repository("https://github.com/PerishLab/plumb.git").unwrap(),
        "PerishLab/plumb"
    );
    assert_eq!(
        repository("git@github.com:PerishLab/plumb.git").unwrap(),
        "PerishLab/plumb"
    );
    assert!(repository("ssh://git@git.perish.top/PerishLab/plumb.git").is_err());
}

const PROMOTION: &str = "aaa\trefs/heads/release/v0.38.0\nt1\trefs/tags/v0.38.0-beta.15\nbbb\trefs/tags/v0.38.0-beta.15^{}\nt2\trefs/tags/v0.38.0-beta.16\naaa\trefs/tags/v0.38.0-beta.16^{}\nt3\trefs/tags/v0.38.0-beta.17\naaa\trefs/tags/v0.38.0-beta.17^{}\n";

fn record(marker: &str, commit: &str, state: &str) -> Vec<u8> {
    format!(r#"{{"marker":"{marker}","commit":"{commit}","state":"{state}"}}"#).into_bytes()
}

#[test]
fn promotion() {
    let found = promoted(PROMOTION, "v0.38.0", "aaa", |_, beta| {
        Ok((beta == "v0.38.0-beta.16").then(|| record(beta, "aaa", "complete")))
    });
    assert_eq!(found.as_deref(), Ok("v0.38.0-beta.16"));
}

#[test]
fn unshipped() {
    assert!(
        promoted(PROMOTION, "v0.38.0", "ccc", |_, _| Ok(None))
            .unwrap_err()
            .contains("no rc or beta marker stands")
    );
    assert!(
        promoted(PROMOTION, "v0.38.0", "aaa", |_, _| Ok(None))
            .unwrap_err()
            .contains("has completed its distribution")
    );
    let moved = promoted(PROMOTION, "v0.38.0", "aaa", |_, beta| {
        Ok(Some(record(beta, "bbb", "complete")))
    });
    assert!(
        moved
            .unwrap_err()
            .contains("has completed its distribution")
    );
    let unfinished = promoted(PROMOTION, "v0.38.0", "aaa", |_, beta| {
        Ok(Some(record(beta, "aaa", "incomplete")))
    });
    assert!(
        unfinished
            .unwrap_err()
            .contains("has completed its distribution")
    );
}

#[test]
fn preference() {
    let listing =
        format!("{PROMOTION}t4\trefs/tags/v0.38.0-rc.1\naaa\trefs/tags/v0.38.0-rc.1^{{}}\n");
    let found = promoted(&listing, "v0.38.0", "aaa", |_, marker| {
        Ok(Some(record(marker, "aaa", "complete")))
    });
    assert_eq!(found.as_deref(), Ok("v0.38.0-rc.1"));
}

fn distributed(run: &str, state: &str, npm: &str) -> Vec<u8> {
    format!(
        r#"{{"marker":"v0.43.0","commit":"abcdef0123456789","state":"{state}","attempt":{{"run":"{run}"}},"media":{{"binaries":"published","npm":"{npm}","oci":"none"}}}}"#
    )
    .into_bytes()
}

#[test]
fn verdict() {
    use super::status::verdict;
    let said = verdict(
        Some(distributed("9", "complete", "published")),
        "v0.43.0",
        "9",
    );
    assert_eq!(
        said.as_deref(),
        Ok(
            "v0.43.0 complete at abcdef012345\n  binaries  published\n  npm       published\n  oci       none"
        )
    );
    let stale = verdict(
        Some(distributed("8", "complete", "published")),
        "v0.43.0",
        "9",
    );
    assert!(stale.unwrap_err().contains("did not write"));
    let unfinished = verdict(
        Some(distributed("9", "incomplete", "failed")),
        "v0.43.0",
        "9",
    );
    assert!(unfinished.unwrap_err().contains("npm       failed"));
    assert!(
        verdict(None, "v0.43.0", "9")
            .unwrap_err()
            .contains("wrote no distribution record")
    );
    assert!(
        verdict(
            Some(distributed("9", "complete", "published")),
            "v0.43.1",
            "9"
        )
        .is_err()
    );
}

#[test]
fn public() {
    use super::status::{public, record};
    let named =
        |npm| public(&record(&distributed("9", "incomplete", npm), "v0.43.0").expect("record"));
    assert_eq!(named("failed"), ["binaries"]);
    assert_eq!(named("present"), ["binaries", "npm"]);
    assert_eq!(named("skipped"), ["binaries", "npm"]);
}
