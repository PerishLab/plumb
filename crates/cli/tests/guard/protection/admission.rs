use super::fixture::{Fixture, action, rejected, success};

fn scope(fixture: &Fixture, repository: &str, features: &str) {
    let root = fixture.home.path().join("overlay/rules/atoms");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("structure.toml"), format!(
        "[lock]\nrule='structure.lock-protection'\n[[lock.scope]]\nrepository={repository:?}\nfeatures={features}\n"
    )).unwrap();
}

#[test]
fn required() {
    let fixture = Fixture::new();
    fixture.policy(false, true);
    success(fixture.run());
    scope(&fixture, "Example/else", "['default','all-features']");
    let unselected = success(fixture.run());
    assert!(
        unselected["actions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|action| action["name"] != "guard/lock")
    );
    scope(&fixture, "example/PROBE", "['default','all-features']");
    let calls = fixture.calls();
    rejected(
        fixture.run(),
        "structure.lock-protection: activated lock scope Example/probe requires guard.lock.enforce=true",
    );
    assert_eq!(fixture.calls(), calls);
    fixture.policy(true, true);
    let active = success(fixture.run());
    assert_eq!(action(&active, "guard/lock")["name"], "guard/lock");
    assert_eq!(success(fixture.run()), active);
    let calls = fixture.calls();
    fixture.policy(false, true);
    rejected(fixture.run(), "requires guard.lock.enforce=true");
    assert_eq!(fixture.calls(), calls);
    fixture.policy(true, true);
    let path = fixture.root.path().join("plumb.toml");
    let original = std::fs::read_to_string(&path).unwrap();
    fixture.put("plumb.toml", original.split("[guard.lock]").next().unwrap());
    fixture.git(&["add", "."]);
    rejected(fixture.run(), "requires guard.lock.enforce=true");
    assert_eq!(fixture.calls(), calls);
    for features in ["[\"default\"]", "[\"all-features\"]"] {
        let narrowed = original.replace("[\"default\",\"all-features\"]", features);
        assert_ne!(narrowed, original);
        fixture.put("plumb.toml", &narrowed);
        fixture.git(&["add", "."]);
        rejected(fixture.run(), "requires its exact carried feature coverage");
        assert_eq!(fixture.calls(), calls);
    }
    fixture.policy(true, false);
    rejected(fixture.run(), "requires both guard/rust and guard/ectropy");
    fixture.policy(true, true);
    assert_eq!(success(fixture.run()), active);
}

#[test]
fn policy() {
    let fixture = Fixture::new();
    fixture.policy(false, false);
    scope(&fixture, "Example/else", "[]");
    rejected(fixture.run(), "explicit unique feature coverage");
    scope(&fixture, "Example/else", "['default','default']");
    rejected(fixture.run(), "explicit unique feature coverage");
    scope(&fixture, "Example/else", "['unknown']");
    rejected(fixture.run(), "invalid carried lock policy");
    scope(&fixture, "Example/else/more", "['default']");
    rejected(fixture.run(), "unique owner/repository identities");
    scope(&fixture, "Example/else", "['default']");
    let path = fixture
        .home
        .path()
        .join("overlay/rules/atoms/structure.toml");
    let duplicate = std::fs::read_to_string(&path).unwrap()
        + "\n[[lock.scope]]\nrepository='example/ELSE'\nfeatures=['default']\n";
    std::fs::write(&path, duplicate).unwrap();
    rejected(fixture.run(), "unique owner/repository identities");
    std::fs::write(&path, "[lock]\nrule='foreign'\nscope=[]\n").unwrap();
    rejected(fixture.run(), "names a different rule");
    std::fs::write(
        &path,
        "[lock]\nrule='structure.lock-protection'\nscope=[]\nextra=true\n",
    )
    .unwrap();
    rejected(fixture.run(), "invalid carried lock policy");
    std::fs::write(&path, "[dir]\nmember=[]\n").unwrap();
    rejected(fixture.run(), "carried structure rules need a lock policy");
}
