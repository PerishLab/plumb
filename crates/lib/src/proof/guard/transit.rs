use super::store::Seat;
use super::{Descriptor, TRAILER};
use std::path::{Path, PathBuf};

pub fn stage(root: &Path, proof: &Descriptor) -> Result<PathBuf, String> {
    proof.current(root)?;
    let path = Seat::new(root)?.pending(&proof.tree)?;
    let parent = path
        .parent()
        .ok_or_else(|| "guard proof has no parent seat".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let text = serde_json::to_vec_pretty(proof)
        .map_err(|error| format!("cannot encode guard proof: {error}"))?;
    let temporary = parent.join(format!(".{}.tmp", proof.tree));
    std::fs::write(&temporary, text)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))?;
    Ok(path)
}

pub fn staged(root: &Path, tree: &str) -> Result<Descriptor, String> {
    Seat::new(root)?.staged(tree)
}

pub fn attach(root: &Path, message: &Path) -> Result<Descriptor, String> {
    let tree = super::tree(root)?;
    let proof = staged(root, &tree)?;
    let token = proof.encode()?;
    let mut text = std::fs::read_to_string(message)
        .map_err(|error| format!("cannot read commit message {}: {error}", message.display()))?;
    let carried = text
        .lines()
        .filter_map(|line| line.strip_prefix(TRAILER).map(str::trim))
        .collect::<Vec<_>>();
    if let [carried] = carried.as_slice() {
        let held = Descriptor::decode(carried)?;
        if held == proof {
            return Ok(proof);
        }
        text = text
            .lines()
            .map(|line| {
                line.strip_prefix(TRAILER)
                    .map_or_else(|| line.to_string(), |_| format!("{TRAILER} {token}"))
            })
            .collect::<Vec<_>>()
            .join("\n");
        text.push('\n');
        std::fs::write(message, text).map_err(|error| {
            format!(
                "cannot update commit message {}: {error}",
                message.display()
            )
        })?;
        return Ok(proof);
    }
    if !carried.is_empty() {
        return Err("commit message carries multiple guard proofs".into());
    }
    while text.ends_with('\n') {
        text.pop();
    }
    text.push_str("\n\n");
    text.push_str(TRAILER);
    text.push(' ');
    text.push_str(&token);
    text.push('\n');
    std::fs::write(message, text)
        .map_err(|error| format!("cannot write commit message {}: {error}", message.display()))?;
    Ok(proof)
}
