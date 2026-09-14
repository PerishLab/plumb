mod binding;
mod execute;
#[path = "../native/mod.rs"]
mod native;
mod request;
#[path = "../resolve/mod.rs"]
mod resolve;
mod seal;
mod sources;
mod support;

pub(super) fn execute(request: &str) -> Result<String, String> {
    execute::run(request)
}

pub(super) fn resolve(marker: &str, atom: &str) -> Result<String, String> {
    resolve::run(marker, atom)
}
