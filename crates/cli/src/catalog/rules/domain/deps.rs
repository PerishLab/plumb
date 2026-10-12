use super::super::rules;

rules! {
    RUST_BINARY_USES_CLAP => "deps.rust-binary-uses-clap",
    RUST_BINARY_USES_PLUMB => "deps.rust-binary-uses-plumb",
    CURRENT_DEPENDENCY_NAME => "deps.current-dependency-name",
    SELF_BUILT_DEPENDENCY_UNPINNED => "deps.self-built-dependency-unpinned",
    FIRST_PARTY_STABLE_LATEST => "deps.first-party-stable-latest",
    STYLING_PACKAGE_ALLOWED => "deps.styling-package-allowed",
    LOADER_ABSENT => "deps.environment-loader-absent",
}
