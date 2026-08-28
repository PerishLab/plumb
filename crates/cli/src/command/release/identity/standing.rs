pub(super) fn line(version: &str, refresh: bool) -> String {
    if refresh {
        format!("origin/release/{version}")
    } else {
        "HEAD".into()
    }
}
