use plumb::config::{Contract, Execution};
use plumb::rule::Probe;

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
    assert!(probe.run(&held).expect("bound observation").matches);
    probe.stdout = "false\n".into();
    assert!(!probe.run(&held).expect("mismatch").matches);
    probe.argv[0] = "unbound-tool".into();
    assert!(probe.run(&held).err().unwrap().contains("no resolved tool"));
    probe.argv.clear();
    assert!(probe.run(&held).err().unwrap().contains("nonempty program"));
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
        .observe(&mut probe.command().expect("command"))
        .expect("observe");
    assert!(seen.matches);
    let mut wrong = probe.command().expect("command");
    wrong.arg("extra");
    assert!(
        probe
            .observe(&mut wrong)
            .err()
            .expect("different argv")
            .contains("differs")
    );
}

#[test]
fn bounded() {
    let executable = std::env::current_exe()
        .expect("test binary")
        .to_string_lossy()
        .into_owned();
    let probe = Probe {
        argv: vec![
            executable,
            "--exact".into(),
            "process::child".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ],
        stdout: String::new(),
        platform: None,
    };
    for (mode, expected) in [("volume", "65536"), ("timeout", "5 second")] {
        let mut command = probe.command().expect("command");
        command.env("PLUMB_PROBE_TEST", mode);
        let error = probe.observe(&mut command).err().expect("bounded refusal");
        assert!(error.contains(expected), "{error}");
        let root = tempfile::tempdir().expect("execution root");
        let held = execution(root.path(), &probe.argv[0], Some(mode));
        let error = probe.run(&held).err().expect("bound refusal");
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
        "timeout" => std::thread::sleep(std::time::Duration::from_secs(30)),
        _ => panic!("unknown mode"),
    }
}
