use super::run;
use plumb::depot::v3::{Bundle, Identity, Kind, Marker, Pointer, Publication};
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Command;

pub(super) const MARKER: &str = "v0.37.43-beta.1";

pub(super) struct World {
    pub(super) root: tempfile::TempDir,
    pub(super) bare: tempfile::TempDir,
    pub(super) home: tempfile::TempDir,
    pub(super) source: String,
    pub(super) bundle: Bundle,
}

fn git(root: &Path, args: &[&str]) -> String {
    String::from_utf8(run(Command::new("git").arg("-C").arg(root).args(args)).stdout)
        .unwrap()
        .trim()
        .to_string()
}

impl World {
    pub(super) fn new() -> Self {
        Self::create(false)
    }

    pub(super) fn bound() -> Self {
        Self::create(true)
    }

    fn create(bound: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let bare = tempfile::tempdir().unwrap();
        let media = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let server = super::super::lifecycle::Server::new(root.path());
        let source = server.source;
        git(bare.path(), &["init", "--bare", "-q"]);
        git(root.path(), &["init", "-q"]);
        git(root.path(), &["config", "user.name", "Fixture"]);
        git(
            root.path(),
            &["config", "user.email", "fixture@example.test"],
        );
        git(
            root.path(),
            &[
                "remote",
                "add",
                "origin",
                "ssh://git@git.perish.top/PerishFire/plumb.git",
            ],
        );
        let ssh = bare.path().join("transport");
        std::fs::write(
            &ssh,
            format!(
                "#!/bin/sh\nexec git-upload-pack '{}'\n",
                bare.path().display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o755)).unwrap();
        git(
            root.path(),
            &["config", "core.sshCommand", ssh.to_str().unwrap()],
        );
        git(root.path(), &["config", "ssh.variant", "simple"]);
        git(
            root.path(),
            &[
                "remote",
                "set-url",
                "--push",
                "origin",
                bare.path().to_str().unwrap(),
            ],
        );
        let datum = root.path().join(plumb::datum::leaf("v0.37.43"));
        std::fs::create_dir_all(datum.parent().unwrap()).unwrap();
        std::fs::write(datum, "schema = 1\nversion = 'v0.37.43'\n").unwrap();
        git(root.path(), &["add", "."]);
        let tree = git(root.path(), &["write-tree"]);
        let proof = crate::marker::proof(&tree, "PerishFire/plumb");
        git(
            root.path(),
            &[
                "commit",
                "-qm",
                &format!("fixture\n\nPlumb-Guard-Proof: {proof}"),
            ],
        );
        git(
            root.path(),
            &["tag", "-a", MARKER, "-m", &format!("plumb {MARKER}")],
        );
        git(
            root.path(),
            &[
                "push",
                "-q",
                "origin",
                "HEAD:refs/heads/main",
                "HEAD:refs/heads/release/v0.37.43",
                "--tags",
            ],
        );
        let manifest = format!(
            "[release]\nproduct='plumb'\nauthority='https://releases.test'\nbinaries=['plumb']\ntargets=['x86_64-unknown-linux-gnu']\n[release.depot]\nsource={source:?}\nderivatives=['configuration']\nvalidator=['plumb','doctor']\n"
        );
        let profile = format!(
            "schema='plumb.product-profile/v1'\n[product]\nname='plumb'\nauthority='https://releases.test'\ndepot='https://depot.test'\nderivatives=['configuration']\n[governance]\nmanifest={manifest:?}\nectropy='[comment]'\n"
        );
        let digest = plumb::depot::sha(profile.as_bytes());
        std::fs::create_dir_all(media.path().join("profiles")).unwrap();
        std::fs::create_dir_all(media.path().join("rules")).unwrap();
        std::fs::write(
            media.path().join(format!("profiles/{digest}.toml")),
            profile,
        )
        .unwrap();
        std::fs::write(media.path().join("rules/products.toml"), format!("schema='plumb.products/v2'\n[[product]]\nidentity='git.perish.top/PerishFire/plumb'\nprofile='{digest}'\n")).unwrap();
        let running = plumb::version!("PLUMB").to_string();
        let seed = Bundle::read(
            media.path(),
            Identity {
                product: "plumb".into(),
                channel: "stable".into(),
                version: running.clone(),
                marker: Marker {
                    name: running,
                    sha256: "a".repeat(64),
                },
                kind: Kind::Configuration,
            },
        )
        .unwrap();
        let pointer = Pointer::new(
            &seed.manifest,
            Publication {
                source: &source,
                prior: None,
                created: "2026-09-16T00:00:00Z".into(),
            },
        )
        .unwrap();
        plumb::depot::v3::install(&home.path().join("configurations"), &pointer, &seed).unwrap();
        if bound {
            let annotation = serde_json::json!({"schema":"plumb.release-marker/v3", "product":"plumb", "marker":MARKER,
                "configuration":{"channel":"stable","version":seed.manifest.version,"generation":seed.manifest.generation().unwrap()},
                "profile":digest});
            git(
                root.path(),
                &["tag", "-f", "-a", MARKER, "-m", &annotation.to_string()],
            );
            git(
                root.path(),
                &["push", "-q", "-f", "origin", &format!("refs/tags/{MARKER}")],
            );
        }
        let output = run(Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(root.path())
            .env("PLUMB_HOME", home.path())
            .args(["release", "verify", "--marker", MARKER]));
        let text = String::from_utf8(output.stdout).unwrap();
        let digest = text
            .trim()
            .rsplit_once('(')
            .unwrap()
            .1
            .trim_end_matches(')');
        let bundle = Bundle::read(
            media.path(),
            Identity {
                product: "plumb".into(),
                channel: "beta".into(),
                version: MARKER.into(),
                marker: Marker {
                    name: MARKER.into(),
                    sha256: digest.into(),
                },
                kind: Kind::Configuration,
            },
        )
        .unwrap();
        let held = Self {
            root,
            bare,
            home,
            source,
            bundle,
        };
        held.publish();
        held.objects(&seed);
        held
    }

    pub(super) fn publish(&self) {
        self.objects(&self.bundle);
    }

    fn objects(&self, bundle: &Bundle) {
        let route = plumb::depot::v3::Route::new(
            &bundle.manifest.channel,
            Kind::Configuration,
            &bundle.manifest.version,
        );
        let base = self.root.path().join("depot").join(
            plumb::depot::v3::generation(route, &bundle.manifest.generation().unwrap()).unwrap(),
        );
        for (path, body) in &bundle.bodies {
            let target = base.join("objects").join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, body).unwrap();
        }
        std::fs::write(
            base.join(plumb::depot::v3::LEAF),
            bundle.manifest.encode().unwrap(),
        )
        .unwrap();
    }

    pub(super) fn install(&self, home: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command
            .current_dir(self.root.path())
            .env("PLUMB_HOME", home)
            .env("PLUMB_RULES_SOURCE", &self.source)
            .args([
                "configuration",
                "install",
                "--marker",
                MARKER.strip_prefix('v').unwrap(),
                "--generation",
                &self.bundle.manifest.generation().unwrap(),
                "--path",
            ])
            .arg(home.join("configurations"));
        command
    }
}
