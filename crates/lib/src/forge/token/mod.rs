use super::model::Remote;
use std::path::PathBuf;

mod parser;

use parser::login;

pub fn read(remote: &Remote) -> Result<String, String> {
    let inline = super::settings::token()?;
    if !inline.is_empty() {
        return Ok(inline);
    }
    let mut seats = seats()?;
    for path in &seats {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if let Some(token) = login(&text, &remote.host) {
            return Ok(token);
        }
    }
    seats.sort();
    Err(format!(
        "forgejo token for {} not found in {}; run tea login add --url {}://{}",
        remote.host,
        seats
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        remote.scheme,
        remote.host
    ))
}

fn seats() -> Result<Vec<PathBuf>, String> {
    let home =
        crate::config::home().ok_or_else(|| "HOME is required to locate tea config".to_string())?;
    let mut found = Vec::new();
    if cfg!(target_os = "macos") {
        found.push(
            home.join("Library")
                .join("Application Support")
                .join("tea")
                .join("config.yml"),
        );
    }
    found.push(home.join(".config").join("tea").join("config.yml"));
    found.push(home.join(".tea").join("tea.yml"));
    Ok(found)
}
