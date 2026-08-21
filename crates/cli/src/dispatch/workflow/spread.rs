use crate::shape;

pub fn spread(held: &shape::workflow::Held, root: &str) -> Option<String> {
    let found = held.contribution(root);
    if root.starts_with("key://") {
        return Some(format!("{} paths", found.len()));
    }
    if root.starts_with("suite://") {
        return Some(found.join(" "));
    }
    None
}
