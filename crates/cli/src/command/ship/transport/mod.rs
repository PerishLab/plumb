mod binding;
mod execute;
mod production;
mod promotion;
mod resolve;
mod reuse;
mod seal;
mod sources;
mod support;

pub(super) fn execute(request: &str) -> Result<String, String> {
    execute::run(request)
}

pub(super) fn resolve(raw: &str, atom: &str) -> Result<String, String> {
    if atom.len() != 40 || !atom.bytes().all(|held| held.is_ascii_hexdigit()) {
        return Err("--atom must be one full Git commit".into());
    }
    let marker = crate::command::release::snapshot(raw)?;
    promotion::verify(&marker)?;
    resolve::graph(&marker)
}
