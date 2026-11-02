use super::*;

fn update(root: &Path, version: &str) {
    put(
        root,
        "observer/Cargo.toml",
        &format!("[package]\nname=\"observer\"\nversion=\"{version}\"\nedition=\"2024\"\n"),
    );
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    put(
        root,
        "Cargo.lock",
        &lock.replace(
            "name=\"observer\"\nversion=\"0.0.0\"",
            &format!("name=\"observer\"\nversion=\"{version}\""),
        ),
    );
    stage(root);
}

#[test]
fn dependency() {
    let root = plant();
    let home = tempfile::tempdir().unwrap();
    let path = root.path();
    let workspace = std::fs::read_to_string(path.join("Cargo.toml")).unwrap();
    put(
        path,
        "Cargo.toml",
        &workspace.replace("\"carrier\"]", "\"carrier\",\"observer\"]"),
    );
    let lock = std::fs::read_to_string(path.join("Cargo.lock")).unwrap();
    put(
        path,
        "Cargo.lock",
        &format!("{lock}[[package]]\nname=\"observer\"\nversion=\"0.0.0\"\n"),
    );
    put(path, "observer/src/lib.rs", "pub fn observe() {}\n");
    update(path, "0.0.0");
    let fixture = Fixture {
        root: path,
        home: home.path(),
    };
    let default = success(fixture.run(&["resolve"]));
    let all = success(fixture.run(&["resolve", "--all-features"]));
    let mut references = identities(&default);
    references.extend(identities(&all));
    let reviews = references.iter().map(|(id, files)| {
        json!({"reference":id,"role":"interoperability","protocol":"native interop fixture","hold":"fixture handle lifetime","release":"explicit cleanup","verification":"fixture assertions","evidence":files})
    }).collect::<Vec<_>>();
    let at = default["argv"]
        .as_array()
        .unwrap()
        .iter()
        .position(|arg| arg == "--target")
        .unwrap();
    let valid = json!({"target":default["argv"][at+1],"features":["default","all-features"],"review":reviews});
    fixture.policy(&valid);
    let first = success(fixture.run(&["check"]));
    assert_eq!(first["schema"], "plumb.lock-review/v2");
    assert_eq!(first["references"].as_array().unwrap().len(), 7);
    update(path, "0.0.1");
    let changed = success(fixture.run(&["check"]));
    assert_eq!(changed["references"], first["references"]);
    assert_eq!(changed["declaration"], first["declaration"]);
    for (before, after) in first["reports"]
        .as_array()
        .unwrap()
        .iter()
        .zip(changed["reports"].as_array().unwrap())
    {
        assert_ne!(before["tree"], after["tree"]);
        assert_ne!(before["lockfile"], after["lockfile"]);
        assert_eq!(identities(before), identities(after));
        assert_eq!(
            after["lockfile"],
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(path.join("Cargo.lock")).unwrap())
            )
        );
    }
    let mut strict = valid;
    strict["review"][0]["evidence"]["Cargo.lock"] = changed["reports"][0]["lockfile"].clone();
    fixture.policy(&strict);
    success(fixture.run(&["check"]));
    let lock = std::fs::read_to_string(path.join("Cargo.lock")).unwrap();
    put(path, "Cargo.lock", &format!("{lock}\n# changed snapshot\n"));
    stage(path);
    rejected(fixture.run(&["check"]), "evidence is stale");
}
