mod guard;

use crate::dispatch::release::model::Spec;
use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::path::Path;

const GUARD: &str = include_str!("../../assets/guard/lane.yml.in");
const TOOL: &str = include_str!("../../assets/guard/tool.yml.in");
const PACKAGES: &str = include_str!("../../assets/guard/packages.yml.in");
const PROOF: &str = include_str!("../../assets/guard/proof.yml.in");
const ASK: &str = include_str!("../../assets/guard/ask.yml.in");
const SHIP: &str = include_str!("../../assets/ship/lane.yml.in");
const CARRIED: &str = include_str!("../../assets/ship/binary.yml.in");
const PROJECTED: &str = include_str!("../../assets/ship/project.yml.in");
const INSTALL: &str = include_str!("../../assets/ship/install.yml.in");
const WINDOWS: &str = include_str!("../../assets/ship/windows.yml.in");
const CAPSULE: &str = include_str!("../../assets/ship/capsule.yml.in");
const EXACT: &str = include_str!("../../assets/release/exact.yml.in");
const STABLE: &str = include_str!("../../assets/release/stable.yml.in");
const TOOLS: [&str; 2] = ["ectropy", "plumb"];

pub struct Seat<'a>(pub &'a Path);

fn fill(text: &str, vars: &BTreeMap<&str, String>) -> Result<String, String> {
    plumb::fill::actions(text, vars).map_err(|error| error.to_string())
}

pub struct Lane {
    pub path: String,
    pub rendered: String,
    pub found: Option<String>,
}

impl Lane {
    pub fn drifted(&self) -> bool {
        self.found.as_deref() != Some(self.rendered.as_str())
    }

    pub fn absent(&self) -> bool {
        self.found.is_none()
    }
}

impl Seat<'_> {
    pub fn render(&self) -> Result<Vec<Lane>, String> {
        let spec = Spec::read(&self.0.join("plumb.toml"))?;
        let mut lanes = vec![self.guard(&spec)?];
        if !spec.surface().is_empty() {
            lanes.push(self.ship(&spec)?);
            lanes.push(self.thin("exact.release.yml", EXACT)?);
            lanes.push(self.thin("stable.release.yml", STABLE)?);
        }
        let refused = lanes
            .iter()
            .flat_map(|lane| super::forge::refusals(&lane.path, &lane.rendered))
            .collect::<Vec<_>>();
        if refused.is_empty() {
            Ok(lanes)
        } else {
            Err(refused.join("; "))
        }
    }

    fn ship(&self, spec: &Spec) -> Result<Lane, String> {
        let media = spec.surface();
        let carried = media.contains(&"binary");
        let projected = media.iter().any(|medium| *medium != "binary");
        let mut vars = BTreeMap::from([
            ("forge", crate::rules::RULES.release.forge.clone()),
            ("plumb", manager("plumb")),
            ("after", if carried { ", seal" } else { "" }.to_string()),
        ]);
        let held = fill(INSTALL, &vars)?;
        vars.insert("carry", matrixed(&held, &vars)?);
        vars.insert("install", held);
        let binary = if carried {
            fill(CARRIED, &vars)?
        } else {
            String::new()
        };
        vars.insert(
            "capsule",
            if carried {
                CAPSULE.to_string()
            } else {
                String::new()
            },
        );
        let project = if projected {
            fill(PROJECTED, &vars)?
        } else {
            String::new()
        };
        vars.insert("binary", binary);
        vars.insert("project", project);
        self.seat("ship.yml", fill(SHIP, &vars)?)
    }

    fn thin(&self, name: &str, template: &str) -> Result<Lane, String> {
        self.seat(name, fill(template, &BTreeMap::new())?)
    }

    fn seat(&self, name: &str, rendered: String) -> Result<Lane, String> {
        let path = format!(".forgejo/workflows/{name}");
        Ok(Lane {
            found: std::fs::read_to_string(self.0.join(&path))
                .ok()
                .map(|text| text.replace("\r\n", "\n")),
            path,
            rendered,
        })
    }

    pub fn write(&self, lanes: &[Lane]) -> Result<Vec<String>, String> {
        let mut written = Vec::new();
        for lane in lanes.iter().filter(|lane| lane.drifted()) {
            let path = self.0.join(&lane.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            std::fs::write(&path, &lane.rendered)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
            written.push(lane.path.clone());
        }
        Ok(written)
    }

    pub fn stale(&self) -> Vec<String> {
        let Ok(lanes) = self.render() else {
            return Vec::new();
        };
        lanes
            .iter()
            .filter(|lane| lane.drifted() && !lane.absent())
            .map(|lane| lane.path.clone())
            .collect()
    }

    pub fn drift(&self) -> Vec<String> {
        let Ok(lanes) = self.render() else {
            return Vec::new();
        };
        lanes
            .iter()
            .filter(|lane| lane.drifted())
            .map(|lane| {
                let state = if lane.absent() {
                    "is absent"
                } else {
                    "was hand-edited or rendered by an older Plumb"
                };
                format!("lane {} {state}; run plumb lane --write", lane.path)
            })
            .collect()
    }

    fn guard(&self, spec: &Spec) -> Result<Lane, String> {
        let path = ".forgejo/workflows/guard.yml";
        let vars = BTreeMap::from([
            ("forge", crate::rules::RULES.release.forge.clone()),
            ("env", self.env(spec)),
            ("steps", self.steps(spec)?),
        ]);
        let rendered = fill(GUARD, &vars)?;
        Ok(Lane {
            path: path.to_string(),
            found: std::fs::read_to_string(self.0.join(path))
                .ok()
                .map(|text| text.replace("\r\n", "\n")),
            rendered,
        })
    }

    fn env(&self, spec: &Spec) -> String {
        let seat = guard::Seat::read(self.0);
        if seat.any(&self.proofs(spec)) {
            return guard::LOCK.to_string();
        }
        String::new()
    }

    fn steps(&self, spec: &Spec) -> Result<String, String> {
        let seat = guard::Seat::read(self.0);
        let listed = self.proofs(spec);
        let mut blocks = Vec::new();
        let guarded = seat.any(&listed);
        for tool in TOOLS {
            if spec.product == tool && !guarded {
                continue;
            }
            let vars = BTreeMap::from([("title", title(tool)), ("manager", manager(tool))]);
            blocks.push(fill(TOOL, &vars)?);
        }
        if guarded {
            blocks.push(ASK.to_string());
        }
        if self.0.join("pnpm-lock.yaml").is_file() {
            let vars = BTreeMap::from([("when", seat.when("web"))]);
            blocks.push(fill(PACKAGES, &vars)?);
        }
        blocks.push(seat.steps(listed, PROOF)?);
        Ok(blocks.join("\n"))
    }

    fn proofs(&self, spec: &Spec) -> Vec<guard::Proof> {
        let mut lines = Vec::new();
        let mut push = |key, line: String| lines.push(guard::Proof { key, line });
        if self.0.join("Cargo.toml").is_file() {
            push("rust", "cargo fmt --all --check".to_string());
            push(
                "rust",
                "cargo clippy --all-targets -- -D warnings".to_string(),
            );
            push(
                "rust",
                "cargo check --locked --workspace --all-targets --release".to_string(),
            );
            push("test", "cargo test --locked".to_string());
        }
        if self.0.join("pnpm-lock.yaml").is_file() {
            if self.0.join("biome.json").is_file() {
                push("web", "pnpm biome ci .".to_string());
            }
            push("web", "pnpm -r exec tsc --noEmit".to_string());
            push("web", "pnpm -r test".to_string());
        }
        for name in self.sites() {
            push("web", format!("pnpm --filter {name} build"));
        }
        for tool in TOOLS.iter().rev() {
            push(tool, format!("{} .", invocation(spec, tool)));
        }
        lines
    }

    fn sites(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.0.join("apps")) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for entry in entries.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path().join("package.json")) else {
                continue;
            };
            let Ok(doc) = serde_json::from_str::<Json>(&text) else {
                continue;
            };
            let built = doc.pointer("/scripts/build").is_some();
            let Some(name) = doc.get("name").and_then(Json::as_str) else {
                continue;
            };
            if built {
                found.push(name.to_string());
            }
        }
        found.sort();
        found
    }
}

fn invocation(spec: &Spec, tool: &str) -> String {
    if spec.product != tool {
        return match tool {
            "plumb" => "plumb doctor".to_string(),
            held => held.to_string(),
        };
    }
    let binary = spec.binaries.first().map_or(tool, String::as_str);
    let deed = if tool == "plumb" { " doctor" } else { "" };
    format!("cargo run --quiet --locked --bin {binary} --{deed}")
}

fn matrixed(install: &str, vars: &BTreeMap<&str, String>) -> Result<String, String> {
    let guarded = install.replacen(
        "        run: |",
        "        if: runner.os != 'Windows'\n        run: |",
        1,
    );
    Ok(format!("{guarded}\n{}", fill(WINDOWS, vars)?))
}

fn manager(tool: &str) -> String {
    format!("https://releases.{tool}.perish.uk")
}

fn title(tool: &str) -> String {
    let mut held = tool.chars();
    match held.next() {
        Some(first) => first.to_uppercase().collect::<String>() + held.as_str(),
        None => String::new(),
    }
}
