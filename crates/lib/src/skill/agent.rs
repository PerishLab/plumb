use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Seat {
    pub agent: String,
    pub path: PathBuf,
}

struct Spot {
    agent: &'static str,
    presence: PathBuf,
    skills: PathBuf,
}

pub fn seats(home: &Path, name: &str) -> Vec<Seat> {
    let mut found: Vec<Seat> = Vec::new();
    for spot in spots(home) {
        if !spot.presence.is_dir() && !spot.skills.is_dir() {
            continue;
        }
        let path = spot.skills.join(name);
        if found.iter().any(|held| held.path == path) {
            continue;
        }
        found.push(Seat {
            agent: spot.agent.to_string(),
            path,
        });
    }
    found
}

pub fn named(path: &Path, name: &str) -> bool {
    path.file_name().and_then(|part| part.to_str()) == Some(name)
}

fn spots(home: &Path) -> Vec<Spot> {
    let claude = home.join(".claude");
    let codex = home.join(".codex");
    let shared = home.join(".agents").join("skills");
    let opencode = home.join(".config").join("opencode");
    vec![
        Spot {
            agent: "claude",
            skills: claude.join("skills"),
            presence: claude,
        },
        Spot {
            agent: "codex",
            skills: codex.join("skills"),
            presence: codex,
        },
        Spot {
            agent: "shared",
            presence: shared.clone(),
            skills: shared,
        },
        Spot {
            agent: "opencode",
            skills: opencode.join("skills"),
            presence: opencode,
        },
    ]
}
