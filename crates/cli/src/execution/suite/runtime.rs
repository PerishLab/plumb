use std::path::Path;
use std::process::Command;

pub(super) struct Fixture(pub tempfile::TempDir);

impl Fixture {
    pub fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    pub fn root(&self) -> &Path {
        self.0.path()
    }

    pub fn write(&self, path: &str, text: &str) {
        let path = self.root().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    pub fn command(root: &Path) -> Result<Command, String> {
        let mut command = Command::new("cargo");
        command
            .current_dir(root)
            .env("CARGO_TARGET_DIR", root.join("target"));
        command.env("CARGO_NET_OFFLINE", "true");
        Ok(command)
    }

    pub fn lock(&self) {
        assert!(
            Self::command(self.root())
                .unwrap()
                .arg("generate-lockfile")
                .status()
                .unwrap()
                .success()
        );
    }

    pub fn run(&self) -> Result<(), String> {
        let _held = super::super::signal::Hold::new()?;
        let carrier = Self::carrier();
        super::Runner {
            carrier: &carrier,
            command: &|| Self::command(self.root()),
            cores: 2,
        }
        .execute(self.root())
    }

    pub fn carrier() -> Vec<String> {
        vec![
            std::env::current_exe()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            "--exact".into(),
            "execution::capture::tests::record".into(),
            "--nocapture".into(),
            "--test-threads=1".into(),
            "--".into(),
        ]
    }

    pub fn single(&self) {
        self.write("Cargo.toml", "[package]\nname='lifetime'\nversion='0.0.0'\nedition='2024'\n[lib]\ntest=false\ndoctest=false\n");
        self.write("src/lib.rs", "");
    }

    #[cfg(target_os = "linux")]
    pub fn controlled(&self) {
        self.write(
            "Cargo.toml",
            "[workspace]\nmembers=['a','b','c']\nresolver='2'\n",
        );
        for name in ["a", "b", "c"] {
            self.write(&format!("{name}/Cargo.toml"), &format!("[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n[lib]\ndoctest=false\n"));
            self.write(
                &format!("{name}/src/lib.rs"),
                &format!(
                    r#"
#[test]
fn controlled() {{
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = directory.parent().unwrap();
    std::fs::write(root.join("{name}.start"), std::process::id().to_string()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !root.join("{name}.release").exists() {{
        assert!(std::time::Instant::now() < deadline, "fixture controller did not release {name}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }}
    std::fs::write(root.join("{name}.end"), "done").unwrap();
    assert!(!root.join("{name}.fail").exists(), "deliberate failure");
}}
"#
                ),
            );
        }
        self.lock();
    }
}

#[test]
#[cfg(target_os = "linux")]
fn context() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers=['shared','alpha','beta']\ndefault-members=['alpha','beta']\nresolver='2'\n");
    fixture.write("shared/Cargo.toml", "[package]\nname='shared'\nversion='0.0.0'\nedition='2024'\n[features]\nleft=[]\nright=[]\n");
    fixture.write("shared/src/lib.rs", "pub fn mask()->u8 { u8::from(cfg!(feature=\"left\")) + 2*u8::from(cfg!(feature=\"right\")) }");
    for (name, feature, peer) in [("alpha", "left", "beta"), ("beta", "right", "alpha")] {
        fixture.write(&format!("{name}/Cargo.toml"), &format!("[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nshared={{path='../shared',features=['{feature}']}}\n[lib]\ntest=true\n[[bin]]\nname='helper_{name}'\npath='src/main.rs'\ntest=false\n"));
        fixture.write(
            &format!("{name}/build.rs"),
            "fn main(){println!(\"cargo:rustc-env=BUILD_STAMP=script-value\");}",
        );
        fixture.write(&format!("{name}/src/main.rs"), "fn main(){}");
        fixture.write(&format!("{name}/src/lib.rs"), "/// ```\n/// assert_eq!(shared::mask(), 3);\n/// ```\n/// ```compile_fail\n/// no_such_function();\n/// ```\npub fn docs() {}\n");
        fixture.write(&format!("{name}/tests/{name}.rs"), &format!(r#"
#[test]
#[cfg(target_os = "linux")]
fn context() {{
    assert_eq!(shared::mask(), 3);
    assert_eq!(std::env::var("BUILD_STAMP").unwrap(), "script-value");
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    assert_eq!(std::env::current_dir().unwrap(), root);
    assert!(std::path::Path::new(&std::env::var("CARGO").unwrap()).is_file());
    assert!(std::process::Command::new(std::env::var("CARGO_BIN_EXE_helper_{name}").unwrap()).status().unwrap().success());
    std::fs::write(root.join("../{name}.start"), "started").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !root.join("../{peer}.start").exists() {{
        assert!(std::time::Instant::now() < deadline, "selectors did not overlap");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }}
}}
"#));
    }
    fixture.lock();
    fixture.run().unwrap();
    fixture.write(
        "alpha/src/lib.rs",
        "/// ```\n/// panic!(\"deliberate doc failure\");\n/// ```\npub fn docs() {}\n",
    );
    assert!(fixture.run().unwrap_err().contains("--doc failed"));
}

#[test]
fn custom() {
    let fixture = Fixture::new();
    fixture.single();
    fixture.write("Cargo.toml", "[package]\nname='custom'\nversion='0.0.0'\nedition='2024'\n[lib]\ntest=false\ndoctest=false\n[[test]]\nname='custom'\nharness=false\n");
    fixture.write("tests/custom.rs", "fn main(){ assert_eq!(std::env::args().count(),1); std::fs::write(\"custom.ran\",\"done\").unwrap(); }");
    fixture.lock();
    fixture.run().unwrap();
    assert!(fixture.root().join("custom.ran").exists());
}

#[test]
#[cfg(target_os = "linux")]
fn libraries() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.root().join("output")).unwrap();
    std::os::unix::fs::symlink("output", fixture.root().join("target")).unwrap();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers=['alpha','beta','gamma']\nresolver='2'\n",
    );
    for name in ["alpha", "beta", "gamma"] {
        fixture.write(
            &format!("{name}/Cargo.toml"),
            &format!("[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n"),
        );
        fixture.write(&format!("{name}/src/lib.rs"), r#"
/// ```
/// assert!(std::path::Path::new("ran").exists(), "docs ran before actual tests");
/// ```
pub fn docs() {}
#[test]
fn program() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write("entered", "yes").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !["alpha","beta","gamma"].iter().all(|name| root.join("..").join(name).join("entered").exists()) {
        assert!(std::time::Instant::now() < deadline, "three libraries did not overlap");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::fs::write("ran", "done").unwrap();
}
"#);
    }
    fixture.lock();
    let carrier = Fixture::carrier();
    let _held = super::super::signal::Hold::new().unwrap();
    super::Runner {
        command: &|| Fixture::command(fixture.root()),
        cores: 12,
        carrier: &carrier,
    }
    .execute(fixture.root())
    .unwrap();
    for name in ["alpha", "beta", "gamma"] {
        assert!(fixture.root().join(name).join("ran").exists());
    }
}
