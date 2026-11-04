use super::super::model::Target;
use super::super::runtime::Fixture;
use super::*;
use std::time::Duration;

fn programs(root: &Path) -> Vec<Program> {
    for name in ["a", "b"] {
        std::fs::create_dir_all(root.join(format!("{name}/src"))).unwrap();
        std::fs::write(root.join(format!("{name}/src/lib.rs")), "").unwrap();
    }
    ["a", "b"]
        .into_iter()
        .map(|name| Program {
            package: name.into(),
            name: name.into(),
            executable: root.join(name),
            directory: root.to_path_buf(),
            features: Default::default(),
            custom: false,
            target: Target {
                name: name.into(),
                kind: vec!["lib".into()],
                source: root.join(format!("{name}/src/lib.rs")),
                test: true,
                doctest: true,
                required: Default::default(),
            },
        })
        .collect()
}

fn completed() -> Vec<Option<Completed>> {
    [9, 1]
        .into_iter()
        .enumerate()
        .map(|(index, milliseconds)| {
            Some(Completed {
                index,
                threads: 12,
                queued: Duration::ZERO,
                elapsed: Duration::from_millis(milliseconds),
                result: Ok(()),
            })
        })
        .collect()
}

#[test]
fn lifecycle() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let mut programs = programs(root.path());
    let history = History::new(root.path(), &home, 12, &programs).unwrap();
    assert_eq!(history.schedule().unwrap().slots, 1);
    history.record(&completed()).unwrap();
    programs[0].executable = root.path().join("another-build");
    let history = History::new(root.path(), &home, 12, &programs).unwrap();
    assert_eq!(history.schedule().unwrap().grants, [11, 1]);
    assert_eq!(
        History::new(root.path(), &home, 8, &programs)
            .unwrap()
            .schedule()
            .unwrap()
            .slots,
        1
    );
    programs[0].features.insert("changed".into());
    assert_eq!(
        History::new(root.path(), &home, 12, &programs)
            .unwrap()
            .schedule()
            .unwrap()
            .slots,
        1
    );
    assert_eq!(
        History::new(root.path(), &home, 12, &programs[..1])
            .unwrap()
            .schedule()
            .unwrap()
            .slots,
        1
    );
    let bytes = std::fs::read(&history.path).unwrap();
    let mut failed = completed();
    failed[0].as_mut().unwrap().result = Err("deliberate failure".into());
    assert!(history.record(&failed).is_err());
    assert_eq!(std::fs::read(&history.path).unwrap(), bytes);
}

#[test]
fn foreign() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let programs = programs(root.path());
    let history = History::new(root.path(), &home, 12, &programs).unwrap();
    history.record(&completed()).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&history.path).unwrap()).unwrap();
    let mut schema = original.clone();
    schema["schema"] = "future".into();
    let mut unknown = original.clone();
    unknown["foreign"] = true.into();
    let mut duplicate = original.clone();
    duplicate["entries"][1] = duplicate["entries"][0].clone();
    for value in [schema, unknown, duplicate] {
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(&history.path, &bytes).unwrap();
        let observed = History::new(root.path(), &home, 12, &programs).unwrap();
        assert!(observed.schedule().is_err());
        assert!(observed.record(&completed()).is_err());
        assert_eq!(std::fs::read(&history.path).unwrap(), bytes);
    }
}
#[test]
#[cfg(target_os = "linux")]
fn learning() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers=['a','b']\nresolver='2'\n",
    );
    for name in ["a", "b"] {
        fixture.write(
            &format!("{name}/Cargo.toml"),
            &format!(
                "[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n[lib]\ndoctest=false\n"
            ),
        );
        fixture.write(&format!("{name}/src/lib.rs"), &format!(r#"
#[test]
fn program() {{
    use std::io::Write;
    let args = std::env::args().collect::<Vec<_>>();
    let threads = args.windows(2).find(|pair| pair[0] == "--test-threads").unwrap()[1].clone();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    if "{name}" == "a" && root.join("change-record").exists() {{
        let path = std::fs::read_dir(root.join("home/state/guard/test")).unwrap().next().unwrap().unwrap().path();
        std::fs::write(path, "foreign").unwrap();
    }}
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(root.join("actual")).unwrap();
    writeln!(log, "{name}:{{threads}}").unwrap();
}}
"#));
    }
    fixture.lock();
    let carrier = Fixture::carrier();
    let _held = crate::execution::signal::Hold::new().unwrap();
    let runner = super::super::Runner {
        home: &fixture.root().join("home"),
        command: &|| Fixture::command(fixture.root()),
        cores: 2,
        carrier: &carrier,
    };
    runner.execute(fixture.root()).unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.root().join("actual")).unwrap(),
        "a:2\nb:2\n"
    );
    runner.execute(fixture.root()).unwrap();
    let log = std::fs::read_to_string(fixture.root().join("actual")).unwrap();
    let mut lines = log.lines().collect::<Vec<_>>();
    lines.sort();
    assert_eq!(lines, ["a:1", "a:2", "b:1", "b:2"]);
    let path = std::fs::read_dir(fixture.root().join("home/state/guard/test"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let known = std::fs::read(&path).unwrap();
    std::fs::write(&path, "foreign").unwrap();
    runner.execute(fixture.root()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "foreign");
    let log = std::fs::read_to_string(fixture.root().join("actual")).unwrap();
    assert!(log.ends_with("a:2\nb:2\n"));
    std::fs::write(&path, known).unwrap();
    fixture.write("change-record", "change during actual execution");
    runner.execute(fixture.root()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "foreign");
    let log = std::fs::read_to_string(fixture.root().join("actual")).unwrap();
    assert_eq!(log.lines().count(), 8);
}

#[test]
fn identities() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let mut programs = programs(root.path());
    programs[1].target = programs[0].target.clone();
    let (history, schedule) = History::prepare(root.path(), &home, 12, &programs);
    assert!(history.is_none());
    assert_eq!(schedule.grants, [12; 2]);
    assert_eq!(schedule.slots, 1);
    assert!(!home.exists());
}

#[test]
#[cfg(unix)]
fn aliases() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let mut programs = programs(root.path());
    let canonical = History::new(root.path(), &home, 12, &programs).unwrap();
    canonical.record(&completed()).unwrap();
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(root.path(), &alias).unwrap();
    for program in &mut programs {
        program.target.source =
            alias.join(program.target.source.strip_prefix(root.path()).unwrap());
        program.directory = alias.clone();
    }
    let home = alias.join("home");
    let history = History::new(&alias, &home, 12, &programs).unwrap();
    assert_eq!(history.keys, canonical.keys);
    assert_eq!(history.path, canonical.path);
    assert_eq!(history.schedule().unwrap().grants, [11, 1]);
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("lib.rs"), "").unwrap();
    programs[0].target.source = outside.path().join("lib.rs");
    assert!(
        History::prepare(root.path(), &home, 12, &programs)
            .0
            .is_none()
    );
}
