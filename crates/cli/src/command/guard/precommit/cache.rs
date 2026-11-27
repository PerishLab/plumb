use plumb::guard::Action;
use std::path::PathBuf;

fn seat(proof: &Action) -> Result<PathBuf, String> {
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot cache guard action: no PLUMB_HOME".to_string())?;
    Ok(home
        .join("proof")
        .join("guard")
        .join("actions")
        .join(format!("{}.json", proof.world)))
}

pub(super) fn contains(proof: &Action) -> bool {
    seat(proof)
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Action>(&bytes).ok())
        .as_ref()
        == Some(proof)
}

pub(super) fn record(proof: &Action) -> Result<(), String> {
    let path = seat(proof)?;
    let parent = path
        .parent()
        .ok_or_else(|| "guard action has no cache parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(proof).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::super::super::workflow::tree::Tree;
    use super::super::{Selection, cargo, suite::Catalog, tree, world::Prepared};
    use plumb::depot::{Rules, sha, v3};
    use std::{env, fs, path::Path, process::Command};

    fn bundle() -> (v3::Bundle, v3::Pinned) {
        let mut objects = Vec::new();
        let mut bodies = std::collections::BTreeMap::new();
        for (path, text) in crate::catalog::carried::FILES {
            let bytes = text.as_bytes().to_vec();
            objects.push(v3::Object {
                path: (*path).into(),
                sha256: sha(&bytes),
                size: bytes.len() as u64,
                media: "application/toml".into(),
                executable: false,
            });
            bodies.insert((*path).into(), bytes);
        }
        let manifest = v3::Manifest::new(
            v3::Identity {
                product: "plumb".into(),
                channel: "stable".into(),
                version: "v0.163.0".into(),
                marker: v3::Marker {
                    name: "v0.163.0".into(),
                    sha256: "a".repeat(64),
                },
                kind: v3::Kind::Configuration,
            },
            objects,
        )
        .unwrap();
        let pointer = v3::Pointer::new(
            &manifest,
            v3::Publication {
                source: "https://depot.plumb.perish.uk",
                prior: None,
                created: "2026-10-28T00:00:00Z".into(),
            },
        )
        .unwrap();
        let pinned = v3::Pinned::pointer(&pointer).unwrap();
        (v3::Bundle { manifest, bodies }, pinned)
    }

    #[test]
    fn pinned() {
        if let Some(root) = env::var_os("PLUMB_PINNED_TEST_ROOT") {
            let (bundle, pinned) = bundle();
            let encoded = pinned.encode().unwrap();
            plumb::depot::install(Rules::resolved(&bundle, pinned.clone()).unwrap()).unwrap();
            let root = Path::new(&root);
            let snapshot = plumb::guard::tree(root).unwrap();
            let index = tree::Index::new(root, &snapshot).unwrap();
            let seat = cargo::seat(root, &snapshot).unwrap();
            let prepared = Prepared::new(root, &index, Selection::Local, seat.as_deref()).unwrap();
            let env = |name| {
                &prepared
                    .checks
                    .iter()
                    .find(|check| check.proof.name == name)
                    .unwrap()
                    .execution
                    .as_ref()
                    .unwrap()
                    .environment
            };
            for name in ["guard/plumb", "guard/ectropy"] {
                assert_eq!(
                    env(name).get("PLUMB_DEPOT_MARK"),
                    Some(pinned.generation.as_str())
                );
                assert_eq!(env(name).get("PLUMB_CONFIGURATION"), Some(encoded.as_str()));
            }
            assert!(env("guard/rust").get("PLUMB_DEPOT_MARK").is_none());
            assert!(env("guard/rust").get("PLUMB_CONFIGURATION").is_none());
            drop(index);

            let manifest = fs::read_to_string(root.join("plumb.toml")).unwrap();
            fs::write(root.join("plumb.toml"), manifest.replace("probe", "plumb")).unwrap();
            tree::git(root, &["add", "plumb.toml"], "hosted fixture").unwrap();
            let snapshot = plumb::guard::tree(root).unwrap();
            let index = tree::Index::new(root, &snapshot).unwrap();
            let seat = cargo::seat(root, &snapshot).unwrap();
            let hosted = Prepared::new(root, &index, Selection::Local, seat.as_deref()).unwrap();
            let held = |name| {
                &hosted
                    .checks
                    .iter()
                    .find(|check| check.proof.name == name)
                    .unwrap()
                    .execution
                    .as_ref()
                    .unwrap()
                    .environment
            };
            assert!(held("guard/plumb").get("PLUMB_DEPOT_MARK").is_none());
            assert!(held("guard/plumb").get("PLUMB_CONFIGURATION").is_none());
            assert_eq!(
                held("guard/ectropy").get("PLUMB_DEPOT_MARK"),
                Some(pinned.generation.as_str())
            );
            assert_eq!(
                held("guard/ectropy").get("PLUMB_CONFIGURATION"),
                Some(encoded.as_str())
            );
            assert!(held("guard/rust").get("PLUMB_DEPOT_MARK").is_none());
            assert!(held("guard/rust").get("PLUMB_CONFIGURATION").is_none());
            return;
        }
        let (home, root) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let root = root.path();
        let git = |args: &[&str]| tree::git(root, args, "pinned fixture").unwrap();
        git(&["init", "-q"]);
        git(&[
            "remote",
            "add",
            "origin",
            "https://github.com/Example/probe.git",
        ]);
        fs::create_dir(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(
            root.join("plumb.toml"),
            "[identity]\nns='probe'\n[workflow.hash.guard]\nrust=['Cargo.toml','Cargo.lock','src']\nplumb=['Cargo.toml','Cargo.lock','src']\nectropy=['src']\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 1 }\n").unwrap();
        assert!(
            Command::new("cargo")
                .args(["generate-lockfile", "--offline"])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
        git(&["add", "."]);
        let test = "command::guard::precommit::cache::tests::pinned";
        let result = Command::new(env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env("PLUMB_PINNED_TEST_ROOT", root)
            .env("PLUMB_HOME", home.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn inferred() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        let git = |args: &[&str]| tree::git(root, args, "inferred fixture").unwrap();
        git(&["init", "-q"]);
        fs::write(root.join("plumb.toml"), "[identity]\nns='hardwear'\n").unwrap();
        fs::write(root.join("README.md"), "# hardware\n").unwrap();
        git(&["add", "."]);
        let first = Tree::read(root, None).unwrap();
        let checks = Catalog {
            tree: &first,
            product: "hardwear",
        }
        .checks()
        .unwrap();
        assert!(checks.iter().any(|check| check.name == "guard/plumb"));
        assert!(!checks.iter().any(|check| check.name == "guard/ectropy"));

        fs::write(
            root.join("plumb.toml"),
            "[identity]\nns='probe'\n[ectropy.limit]\nblock = 4\n",
        )
        .unwrap();
        git(&["add", "plumb.toml"]);
        let next = Tree::read(root, None).unwrap();
        let checks = Catalog {
            tree: &next,
            product: "probe",
        }
        .checks()
        .unwrap();
        let first = checks
            .iter()
            .find(|check| check.name == "guard/ectropy")
            .expect("guard/ectropy inferred from plumb.toml [ectropy]");
        assert!(first.paths.contains(&"plumb.toml".to_string()));

        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        fs::write(
            root.join("plumb.toml"),
            "[identity]\nns='probe'\n[workflow.hash.guard]\nectropy=['src']\n[ectropy.limit]\nblock = 4\n",
        )
        .unwrap();
        git(&["add", "."]);
        let explicit = Tree::read(root, None).unwrap();
        let one = Catalog {
            tree: &explicit,
            product: "probe",
        }
        .checks()
        .unwrap();
        let a = one.iter().find(|c| c.name == "guard/ectropy").unwrap();
        assert!(a.paths.contains(&"plumb.toml".to_string()));

        fs::write(
            root.join("plumb.toml"),
            "[identity]\nns='probe'\n[workflow.hash.guard]\nectropy=['src']\n[ectropy.limit]\nblock = 3\n",
        )
        .unwrap();
        git(&["add", "plumb.toml"]);
        let edited = Tree::read(root, None).unwrap();
        let two = Catalog {
            tree: &edited,
            product: "probe",
        }
        .checks()
        .unwrap();
        let b = two.iter().find(|c| c.name == "guard/ectropy").unwrap();
        assert_ne!(a.input, b.input);
    }
}
