use std::path::Path;
use std::process::Command;

use super::write;

pub(super) const PREVIEW: &str = "[lane]\nrepository = 'PerishLab/crest'\n[lane.app.crest]\npath = 'reviews/crest'\npackage = '@perish/review'\n[lane.app.crest.mapping]\nprovider = 'cfworker'\naccess = 'public'\naccount = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\nresource = 'crest-review'\n[lane.app.crest.binding.preview]\nadaptor = 'static'\ncapabilities = ['inspect', 'deploy']\n[lane.app.crest.binding.preview.authorities]\nauthorization = 'ensign'\npublication = 'wharf'\n";

pub(super) fn preview() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("reviews/crest")).unwrap();
    write(root.path(), "plumb.toml", PREVIEW);
    for file in ["package.json", "wrangler.jsonc"] {
        write(root.path(), &format!("reviews/crest/{file}"), "{}");
    }
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["add", "."]);
    root
}

pub(super) fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
}

pub(super) fn previewed(root: &Path) -> String {
    let home = super::super::support::home();
    let output = super::super::support::plumb()
        .args(["doctor", "--json"])
        .arg(root)
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap();
    let held: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    held["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["code"] == "structure.preview-app")
        .map(|finding| finding["evidence"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn declaration() {
    let root = preview();
    let cases = [
        (PREVIEW.to_owned(), false),
        ("[preview]\n".to_owned(), true),
        ("[preview]\napps = {}\n".to_owned(), true),
        ("preview = 'wrong'\n".to_owned(), true),
        ("[preview]\napp = []\n".to_owned(), true),
        (PREVIEW.replace("access = 'public'", ""), true),
        (format!("{PREVIEW}extra = true\n"), true),
        (PREVIEW.replace("'public'", "'private'"), true),
        (PREVIEW.replace("'cfworker'", "'oci'"), true),
        (PREVIEW.replace("'@perish/review'", "'review*'"), true),
        (
            PREVIEW.replace("'@perish/review'", "'@bad scope/review'"),
            true,
        ),
        (PREVIEW.replace("'@perish/review'", "'review'"), false),
        (format!("{PREVIEW}{}", sibling()), true),
    ];
    for (text, refused) in cases {
        write(root.path(), "plumb.toml", &text);
        let held = previewed(root.path());
        assert_eq!(!held.is_empty(), refused, "{text}\n{held}");
    }
    write(root.path(), "plumb.toml", "[release]\n");
    let held = previewed(root.path());
    assert!(held.is_empty(), "{held}");
}

#[test]
fn traversal() {
    let root = preview();
    for path in [
        "",
        ".",
        "..",
        "../outside",
        "/tmp",
        "reviews//crest",
        "reviews/./crest",
        "reviews/crest/",
        "C:/review",
        "reviews\\crest",
        "missing",
    ] {
        write(
            root.path(),
            "plumb.toml",
            &PREVIEW.replace("reviews/crest", path),
        );
        let held = previewed(root.path());
        assert!(!held.is_empty(), "{path}\n{held}");
    }
}

#[test]
fn tracking() {
    for path in [
        "plumb.toml",
        "reviews/crest/package.json",
        "reviews/crest/wrangler.jsonc",
    ] {
        let root = preview();
        git(root.path(), &["rm", "--cached", path]);
        let held = previewed(root.path());
        assert!(!held.is_empty(), "{path}\n{held}");
        assert!(held.contains("tracked regular file"), "{held}");
    }
}

#[test]
fn duplicate() {
    let root = preview();
    std::fs::create_dir_all(root.path().join("reviews/other")).unwrap();
    for file in ["package.json", "wrangler.jsonc"] {
        write(root.path(), &format!("reviews/other/{file}"), "{}");
    }
    git(root.path(), &["add", "."]);
    let other = sibling();
    for text in [
        other.replace("reviews/crest", "reviews/other"),
        other.replace("@perish/review", "@perish/other"),
    ] {
        write(root.path(), "plumb.toml", &format!("{PREVIEW}{text}"));
        let held = previewed(root.path());
        assert!(
            held.contains("duplicate path or package identity"),
            "{held}"
        );
    }
    write(
        root.path(),
        "plumb.toml",
        &format!(
            "{PREVIEW}{}",
            other
                .replace("reviews/crest", "reviews/other")
                .replace("@perish/review", "@perish/other")
        ),
    );
    assert!(previewed(root.path()).is_empty());
}

fn sibling() -> String {
    PREVIEW
        .split_once("[lane.app.crest]")
        .unwrap()
        .1
        .replace("lane.app.crest", "lane.app.other")
        .replace("path =", "[lane.app.other]\npath =")
}

#[cfg(unix)]
#[test]
fn link() {
    for path in [
        "plumb.toml",
        "reviews/crest/package.json",
        "reviews/crest/wrangler.jsonc",
    ] {
        let root = preview();
        let outside = tempfile::tempdir().unwrap();
        std::fs::rename(root.path().join(path), outside.path().join("source")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("source"), root.path().join(path)).unwrap();
        let held = previewed(root.path());
        assert!(held.contains("traverses a symlink"), "{path}\n{held}");
        git(root.path(), &["add", path]);
        assert!(previewed(root.path()).contains("tracked regular file"));
    }
}

#[cfg(unix)]
#[test]
fn symlink() {
    let root = preview();
    let outside = tempfile::tempdir().unwrap();
    std::fs::rename(
        root.path().join("reviews/crest"),
        outside.path().join("crest"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("crest"),
        root.path().join("reviews/crest"),
    )
    .unwrap();
    let held = previewed(root.path());
    assert!(held.contains("traverses a symlink"), "{held}");
}
