use std::path::Path;

#[allow(dead_code)]
pub fn executable(path: &Path, body: &str) {
    plumb::testing::executable(path, body);
}
