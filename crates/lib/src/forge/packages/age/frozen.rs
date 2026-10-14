use super::super::{fetch, process};
use std::path::Path;

pub(super) fn verify(root: &Path, flags: &[String]) {
    let denied = fetch::run(root, &[]).unwrap_err();
    assert!(denied.contains("MINIMUM_RELEASE_AGE_VIOLATION"), "{denied}");
    fetch::run(root, flags).unwrap();
    let mut install = ["pnpm", "install", "--offline", "--frozen-lockfile"]
        .map(str::to_string)
        .to_vec();
    let denied = process::run(root, "pnpm", &install).unwrap_err();
    assert!(denied.contains("MINIMUM_RELEASE_AGE_VIOLATION"), "{denied}");
    install.extend_from_slice(flags);
    process::run(root, "pnpm", &install).unwrap();
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
