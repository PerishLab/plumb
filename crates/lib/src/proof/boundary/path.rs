use super::{Refusal, refuse};

pub fn parse(path: &str, root: bool) -> Result<String, Refusal> {
    if root && path == "." {
        return Ok(path.to_owned());
    }
    if path.is_empty() || path.starts_with('/') || path.ends_with('/') {
        return invalid(path);
    }
    if path.contains('\0') || path.contains('\\') {
        return invalid(path);
    }
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return invalid(path);
        }
    }
    Ok(path.to_owned())
}

fn invalid<T>(path: &str) -> Result<T, Refusal> {
    Err(refuse(
        "boundary",
        format!("{path:?} is not a canonical Git path"),
    ))
}
