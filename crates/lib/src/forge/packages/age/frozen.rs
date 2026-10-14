use super::super::{fetch, process};
use std::path::Path;

pub(super) fn verify(root: &Path, flags: &[String]) {
    let denied = fetch::run(root, &[]).unwrap_err();
    assert!(denied.contains("MINIMUM_RELEASE_AGE_VIOLATION"), "{denied}");
    let mut policy = flags.to_vec();
    policy.push("--config.verify-deps-before-run=error".into());
    fetch::run(root, &policy).unwrap();
    let mut install = ["pnpm", "install", "--offline", "--frozen-lockfile"]
        .map(str::to_string)
        .to_vec();
    let denied = process::run(root, "pnpm", &install).unwrap_err();
    assert!(denied.contains("MINIMUM_RELEASE_AGE_VIOLATION"), "{denied}");
    install.extend_from_slice(&policy);
    process::run(root, "pnpm", &install).unwrap();
    checks(root, &policy);
    let mut update = [
        "pnpm",
        "update",
        "--lockfile-only",
        "--ignore-scripts",
        "--ignore-pnpmfile",
        "--no-save",
    ]
    .map(str::to_string)
    .to_vec();
    update.extend_from_slice(flags);
    update.extend([
        "--config.minimum-release-age-exclude=aged-third".into(),
        "aged-third".into(),
    ]);
    process::run(root, "packages", &update).unwrap();
    let denied = fetch::run(root, flags).unwrap_err();
    assert!(
        denied.contains("MINIMUM_RELEASE_AGE_VIOLATION") && denied.contains("aged-third@0.2.0"),
        "{denied}"
    );
    let denied = process::run(root, "pnpm", &install).unwrap_err();
    assert!(
        denied.contains("MINIMUM_RELEASE_AGE_VIOLATION") && denied.contains("aged-third@0.2.0"),
        "{denied}"
    );
}

fn checks(root: &Path, policy: &[String]) {
    for tail in [
        vec!["exec", "node", "-e", "console.log(12345)"],
        vec!["run", "check"],
    ] {
        let mut command = vec!["pnpm".into()];
        command.extend_from_slice(policy);
        command.extend(tail.into_iter().map(str::to_string));
        let output = process::run(root, "pnpm", &command).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("12345"));
    }
    let path = root.join("node_modules");
    let held = root.join("held-modules");
    std::fs::rename(&path, &held).unwrap();
    let mut command = vec!["pnpm".into()];
    command.extend_from_slice(policy);
    command.extend(["exec", "node", "-e", "console.log(12345)"].map(str::to_string));
    let denied = process::run(root, "pnpm", &command).unwrap_err();
    assert!(denied.contains("VERIFY_DEPS_BEFORE_RUN"), "{denied}");
    assert!(
        !path.exists(),
        "dependency check must not repair installed state"
    );
    std::fs::rename(held, path).unwrap();
}
