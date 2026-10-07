use std::path::Path;

pub type Evidence = Vec<(&'static str, &'static str, Result<String, String>)>;

pub fn read(root: &Path) -> Evidence {
    let mut tools = Vec::new();
    if root.join("Cargo.toml").is_file() {
        tools.extend(["cargo", "rustc"]);
    }
    if root.join("package.json").is_file() {
        tools.extend(["node", "pnpm"]);
    }
    tools
        .into_iter()
        .filter_map(|tool| {
            crate::consumption::metadata::expected(tool)
                .map(|version| (tool, version, reported(root, tool)))
        })
        .collect()
}

fn reported(root: &Path, tool: &str) -> Result<String, String> {
    let output = plumb::config::detached(tool)
        .arg("--version")
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(detail.lines().next().unwrap_or("").trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace()
        .map(|word| word.trim_start_matches('v'))
        .find(|word| version(word))
        .map(str::to_string)
        .ok_or_else(|| format!("printed {:?}", text.trim()))
}

fn version(word: &str) -> bool {
    let parts = word.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}
