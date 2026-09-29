use serde_json::Value as Json;
use std::path::Path;

pub struct Evidence {
    pub exports: Vec<(String, String, String)>,
    pub tests: Vec<(String, String)>,
}

const SUFFIXES: [&str; 4] = [".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx"];

const ANCHORS: [&str; 2] = ["package.json", "deno.json"];

pub fn read(
    root: &Path,
    snapshot: Result<&plumb::snapshot::Snapshot, &plumb::snapshot::Refusal>,
) -> Evidence {
    let seat = Script(root);
    Evidence {
        exports: seat.exports(),
        tests: snapshot.map(|held| seat.tests(held)).unwrap_or_default(),
    }
}

struct Script<'a>(&'a Path);

impl Script<'_> {
    fn exports(&self) -> Vec<(String, String, String)> {
        let mut found = Vec::new();
        for path in super::node::manifests(self.0) {
            let Some(held) = exported(&path) else {
                continue;
            };
            let seat = path.parent().unwrap_or(self.0);
            let manifest = path
                .strip_prefix(self.0)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            walk(&held, ".", &mut |key, target| {
                if wildcard(seat, target) {
                    found.push((manifest.clone(), key.to_string(), target.to_string()));
                }
            });
        }
        found
    }

    fn tests(&self, snapshot: &plumb::snapshot::Snapshot) -> Vec<(String, String)> {
        let mut found = Vec::new();
        for entry in snapshot.entries() {
            let path = entry.path();
            let test = SUFFIXES.iter().any(|suffix| path.ends_with(suffix));
            let vendored = path.split('/').any(|segment| segment == "node_modules");
            if !test || vendored {
                continue;
            }
            let Some((package, rest)) = self.owner(path) else {
                continue;
            };
            if !rest.starts_with("tests/") {
                found.push((path.to_string(), package));
            }
        }
        found
    }

    fn owner<'p>(&self, path: &'p str) -> Option<(String, &'p str)> {
        let mut parts = path.splitn(3, '/');
        if let (Some(seat @ ("apps" | "packages")), Some(name), Some(rest)) =
            (parts.next(), parts.next(), parts.next())
        {
            let package = format!("{seat}/{name}");
            if anchored(&self.0.join(&package)) {
                return Some((package, rest));
            }
        }
        anchored(self.0).then(|| (".".to_string(), path))
    }
}

fn anchored(dir: &Path) -> bool {
    ANCHORS.iter().any(|name| dir.join(name).is_file())
}

fn exported(path: &Path) -> Option<Json> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut doc = serde_json::from_str::<Json>(&text).ok()?;
    doc.get_mut("exports").map(Json::take)
}

fn walk(value: &Json, key: &str, visit: &mut dyn FnMut(&str, &str)) {
    match value {
        Json::String(target) => visit(key, target),
        Json::Array(list) => list.iter().for_each(|item| walk(item, key, visit)),
        Json::Object(map) => map.iter().for_each(|(name, item)| {
            walk(item, if name.starts_with('.') { name } else { key }, visit)
        }),
        _ => {}
    }
}

fn wildcard(seat: &Path, target: &str) -> bool {
    let Some(star) = target.find('*') else {
        return false;
    };
    let bare = target.trim_start_matches("./");
    if bare.split('/').any(|segment| segment == "src") {
        return true;
    }
    let dir = target[..star].rsplit_once('/').map_or("", |(head, _)| head);
    seat.join(dir).join("src").is_dir()
}
