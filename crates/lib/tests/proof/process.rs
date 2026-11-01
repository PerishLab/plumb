use plumb::config::{Contract, Execution};
use plumb::rule::Probe;
use std::time::Duration;

fn execution(root: &std::path::Path, program: &str, mode: Option<&str>) -> Execution {
    let mut values = std::env::vars_os().collect::<Vec<_>>();
    if let Some(mode) = mode {
        values.push(("PLUMB_PROBE_TEST".into(), mode.into()));
    }
    let environment = Contract {
        inherit: [
            "PATH",
            "SystemRoot",
            "SYSTEMROOT",
            "WINDIR",
            "PATHEXT",
            "PLUMB_PROBE_TEST",
        ]
        .map(str::to_string)
        .to_vec(),
        managed: vec![],
        reject: vec![],
        bind: Default::default(),
    }
    .capture(values)
    .expect("environment");
    Execution::new(environment, &[program.into()], root).expect("execution")
}

#[test]
fn bound() {
    let root = tempfile::tempdir().expect("repository");
    let output = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(root.path())
        .output()
        .expect("init");
    assert!(output.status.success());
    let held = execution(root.path(), "git", None);
    let mut probe = Probe {
        argv: vec![
            "git".into(),
            "rev-parse".into(),
            "--is-inside-work-tree".into(),
        ],
        stdout: "true\r\n".into(),
        platform: None,
    };
    assert!(
        probe
            .run(&held, Probe::LIMIT)
            .expect("bound observation")
            .matches
    );
    probe.stdout = "false\n".into();
    assert!(!probe.run(&held, Probe::LIMIT).expect("mismatch").matches);
    probe.argv[0] = "unbound-tool".into();
    assert!(
        probe
            .run(&held, Probe::LIMIT)
            .err()
            .unwrap()
            .contains("no resolved tool")
    );
    probe.argv.clear();
    assert!(
        probe
            .run(&held, Probe::LIMIT)
            .err()
            .unwrap()
            .contains("nonempty program")
    );
    assert!(
        held.output(&[])
            .err()
            .unwrap()
            .contains("requires a program")
    );
}

#[test]
fn direct() {
    let output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .expect("git");
    let probe = Probe {
        argv: vec!["git".into(), "--version".into()],
        stdout: String::from_utf8(output.stdout).expect("version"),
        platform: None,
    };
    let seen = probe
        .observe(probe.command().expect("command"), Probe::LIMIT)
        .expect("observe");
    assert!(seen.matches);
    let mut wrong = probe.command().expect("command");
    wrong.arg("extra");
    assert!(
        probe
            .observe(wrong, Probe::LIMIT)
            .err()
            .expect("different argv")
            .contains("differs")
    );
}

#[test]
fn bounded() {
    let probe = worker();
    assert_eq!(Probe::LIMIT, Duration::from_secs(10));
    let brief = Duration::from_millis(200);
    for (mode, limit, expected) in [
        ("volume", Probe::LIMIT, "65536"),
        ("timeout", brief, "exceeded 200 millisecond limit"),
    ] {
        let mut command = probe.command().expect("command");
        command.env("PLUMB_PROBE_TEST", mode);
        let error = probe
            .observe(command, limit)
            .err()
            .expect("bounded refusal");
        assert!(error.contains(expected), "{error}");
        let root = tempfile::tempdir().expect("execution root");
        let held = execution(root.path(), &probe.argv[0], Some(mode));
        let error = probe.run(&held, limit).err().expect("bound refusal");
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
#[ignore = "invoked by bounded through the real probe runner"]
fn child() {
    use std::io::Write;
    match std::env::var("PLUMB_PROBE_TEST")
        .expect("child mode")
        .as_str()
    {
        "volume" => std::io::stdout()
            .write_all(&vec![b'x'; 65537])
            .expect("output"),
        "diagnostic" => {
            std::io::stderr().write_all(&vec![b'e'; 2_097_152]).unwrap();
            println!("done");
        }
        "owned" => {
            assert!(std::env::var_os("PATH").is_none());
            println!("{}", std::env::var("PLUMB_OWNED_TEST").unwrap());
        }
        "timeout" => std::thread::sleep(std::time::Duration::from_secs(30)),
        _ => panic!("unknown mode"),
    }
}

fn worker() -> Probe {
    let executable = std::env::current_exe()
        .expect("test binary")
        .to_string_lossy()
        .into_owned();
    Probe {
        argv: vec![
            executable,
            "--exact".into(),
            "process::child".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ],
        stdout: String::new(),
        platform: None,
    }
}

#[test]
fn ownership() {
    let probe = worker();
    let mut command = probe.command().unwrap();
    command
        .env_clear()
        .env("PLUMB_PROBE_TEST", "owned")
        .env("PLUMB_OWNED_TEST", "owned 中文");
    let observed = probe.observe(command, Probe::LIMIT).unwrap();
    assert!(observed.stdout.contains("owned 中文"));
}

#[test]
fn runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let probe = worker();
    let root = tempfile::tempdir().unwrap();
    let held = execution(root.path(), &probe.argv[0], Some("diagnostic"));
    let output = held.output(&probe.argv).unwrap();
    assert!(output.status.success());
    assert!(output.stdout.windows(4).any(|bytes| bytes == b"done"));
    assert!(output.stderr.is_empty());
}
