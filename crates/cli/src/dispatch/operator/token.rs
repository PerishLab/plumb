use super::api::Remote;
use std::path::PathBuf;

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
        plumb::config::home().ok_or_else(|| "HOME is required to locate tea config".to_string())?;
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

fn login(text: &str, host: &str) -> Option<String> {
    let mut entry = Vec::new();
    for raw in text.lines().chain(["- "]) {
        let line = raw.trim();
        if line.starts_with("- ") && !entry.is_empty() {
            if let Some(token) = matching(&entry, host) {
                return Some(token);
            }
            entry.clear();
        }
        if let Some((key, value)) = field(line.trim_start_matches("- ")) {
            entry.push((key.to_string(), value.to_string()));
        }
    }
    None
}

fn matching(entry: &[(String, String)], host: &str) -> Option<String> {
    let value = |name: &str| {
        entry
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
            .unwrap_or("")
    };
    let url = value("url");
    let seen = url
        .split_once("://")
        .map_or(url, |(_, tail)| tail)
        .split('/')
        .next()
        .unwrap_or("");
    let token = value("token");
    if token.is_empty() {
        return None;
    }
    if [seen, value("ssh_host")].contains(&host) {
        Some(token.to_string())
    } else {
        None
    }
}

fn field(line: &str) -> Option<(&str, &str)> {
    let (key, raw) = line.split_once(':')?;
    if !["url", "token", "ssh_host"].contains(&key.trim()) {
        return None;
    }
    let value = raw.trim();
    let held = if quoted(value) {
        &value[1..value.len() - 1]
    } else {
        value
    };
    Some((key.trim(), held))
}

fn quoted(value: &str) -> bool {
    if value.len() < 2 {
        return false;
    }
    matches!(
        (value.chars().next(), value.chars().last()),
        (Some('"'), Some('"')) | (Some('\''), Some('\''))
    )
}
