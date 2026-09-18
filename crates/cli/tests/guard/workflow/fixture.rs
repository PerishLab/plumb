use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct Seat(PathBuf);

pub struct Plan<'a> {
    pub base: Option<&'a str>,
    pub world: &'a [&'a str],
    pub identity: &'a [&'a str],
    pub project: &'a [&'a str],
    pub roots: &'a [&'a str],
    pub inventory: Option<&'a str>,
}

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

    pub fn plan(&self, base: Option<&str>, world: &[&str]) -> (String, bool) {
        self.planned(Plan {
            base,
            world,
            identity: &[],
            project: &[],
            roots: &[],
            inventory: None,
        })
    }

    pub fn planned(&self, input: Plan<'_>) -> (String, bool) {
        self.remote(input, None)
    }

    pub fn remote(&self, input: Plan<'_>, inventory: Option<&str>) -> (String, bool) {
        self.invoke(input, inventory, &[])
    }

    fn invoke(
        &self,
        input: Plan<'_>,
        inventory: Option<&str>,
        workload: &[&str],
    ) -> (String, bool) {
        let mut args = vec!["workflow", "plan"];
        if let Some(base) = input.base {
            args.push("--base");
            args.push(base);
        }
        for entry in input.world {
            args.push("--world");
            args.push(entry);
        }
        for entry in workload {
            args.push("--workload");
            args.push(entry);
        }
        for entry in input.identity {
            args.push("--identity");
            args.push(entry);
        }
        for entry in input.project {
            args.push("--project");
            args.push(entry);
        }
        for entry in input.roots {
            args.push("--root");
            args.push(entry);
        }
        if let Some(inventory) = input.inventory {
            args.push("--inventory");
            args.push(inventory);
        }
        if let Some(inventory) = inventory {
            args.push("--inventory-url");
            args.push(inventory);
        }
        args.push(self.0.to_str().expect("path"));
        let out = plumb(&args).output().expect("run");
        (
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            out.status.success(),
        )
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
