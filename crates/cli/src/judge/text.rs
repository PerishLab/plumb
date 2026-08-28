use std::collections::BTreeSet;

pub const COMPONENTS: &str =
    "packages/components is reserved; reusable components belong to the design system";

pub fn show(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        return "-".to_string();
    }
    set.iter().cloned().collect::<Vec<_>>().join(" ")
}
