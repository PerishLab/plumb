use std::collections::BTreeSet;

pub const CONCURRENCY: &str = "concurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true";
pub const CONTAINER: &str = "mirror.perish.lan/ci/deno";
pub const COMPONENTS: &str =
    "packages/components is reserved; reusable components belong to the design system";

pub fn show(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        return "-".to_string();
    }
    set.iter().cloned().collect::<Vec<_>>().join(" ")
}
