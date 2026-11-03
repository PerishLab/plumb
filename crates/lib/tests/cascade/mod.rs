use radicula_config::Cascade;
mod mint;

#[test]
#[cfg(any(feature = "vendor", feature = "skill"))]
fn harness() {
    let get = |key: &str| match key {
        "HARNESS_RUN_POLL_MS" => Some("250".to_string()),
        "HARNESS_GUARD_TIMEOUT_MS" => Some("9000".to_string()),
        _ => None,
    };
    let held = plumb::rig::Harness::default()
        .merge(plumb::rig::Harness::lookup("HARNESS", &get).expect("harness env"));
    assert_eq!(held.run.poll.duration().as_millis(), 250);
    assert_eq!(held.guard.timeout.seconds(), 9);
}

#[test]
#[cfg(any(feature = "vendor", feature = "skill"))]
fn mapped() {
    let get = |key: &str| match key {
        "PLUMB_RELEASE_REGISTRY_TOKEN" => Some("Bearer held".to_string()),
        "PLUMB_PUBLISH_SECRET_FILE" => Some("secret/path".to_string()),
        _ => None,
    };
    let held =
        plumb::rig::Rig::default().merge(plumb::rig::Rig::lookup("PLUMB", &get).expect("rig env"));
    assert_eq!(held.release.credential, "Bearer held");
    assert_eq!(held.publish.file, std::path::PathBuf::from("secret/path"));
}

#[test]
#[cfg(any(feature = "vendor", feature = "skill"))]
fn inventory() {
    let get = |key: &str| match key {
        "PLUMB_WORKFLOW_INVENTORY_ACCESS" => Some("access".to_string()),
        "PLUMB_WORKFLOW_INVENTORY_SECRET" => Some("secret".to_string()),
        "PLUMB_WORKFLOW_INVENTORY_BUCKET" => Some("bucket".to_string()),
        "PLUMB_WORKFLOW_INVENTORY_ENDPOINT" => Some("https://s3.test".to_string()),
        "PLUMB_WORKFLOW_INVENTORY_URL" => Some("https://inventory.test/inventory.json".to_string()),
        _ => None,
    };
    let held =
        plumb::rig::Rig::default().merge(plumb::rig::Rig::lookup("PLUMB", &get).expect("rig"));
    assert_eq!(held.workflow.inventory.access, "access");
    assert_eq!(held.workflow.inventory.secret, "secret");
    assert_eq!(held.workflow.inventory.bucket, "bucket");
    assert_eq!(held.workflow.inventory.endpoint, "https://s3.test");
    assert_eq!(
        held.workflow.inventory.url,
        "https://inventory.test/inventory.json"
    );
}

#[test]
#[cfg(any(feature = "vendor", feature = "skill"))]
fn authority() {
    let dir = tempfile::tempdir().expect("fixture");
    let file = dir.path().join("secret");
    std::fs::write(&file, "held\n").expect("secret");
    let mut authority = plumb::rig::Authority {
        file: std::path::PathBuf::from(&file),
        ..plumb::rig::Authority::default()
    };
    authority.load().expect("secret should load");
    assert_eq!(authority.secret, "held");
}

#[test]
#[cfg(feature = "skill")]
fn site() {
    let get = |key: &str| match key {
        "PLUMB_SITE_ACCOUNT" => Some("account".to_string()),
        "PLUMB_SITE_BLIND" => Some("true".to_string()),
        "PLUMB_SITE_DELAY" => Some("25".to_string()),
        "PLUMB_SITE_DOMAIN" => Some("site.test".to_string()),
        "PLUMB_SITE_TOKEN" => Some("secret".to_string()),
        "PLUMB_SITE_TURNS" => Some("3".to_string()),
        _ => None,
    };
    let held = plumb::rig::Rig::default()
        .merge(plumb::rig::Rig::lookup("PLUMB", &get).expect("site environment should read"));
    assert_eq!(held.site.account, "account");
    assert!(held.site.blind);
    assert_eq!(held.site.delay, 25);
    assert_eq!(held.site.domain, "site.test");
    assert_eq!(held.site.token, "secret");
    assert_eq!(held.site.turns, 3);
}

#[derive(Default, radicula_config::Cascade)]
#[cascade(section)]
struct Section {
    count: u16,
}

#[derive(Default, plumb::config::Cascade)]
struct Bridge {
    #[cascade(section)]
    section: Section,
}

#[test]
fn bridge() {
    fn root<T: radicula_config::layer::Cascade>() {}
    root::<Bridge>();
    let get = |key: &str| (key == "BRIDGE_SECTION_COUNT").then(|| "17".into());
    let held = Bridge::default().merge(Bridge::lookup("BRIDGE", &get).unwrap());
    assert_eq!(held.section.count, 17);
    assert_eq!(Bridge::prefix(), "PLUMB");
}
