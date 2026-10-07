pub const VERSIONS: [(&str, &str); 3] = [
    ("node.version", "24.18.0"),
    ("pnpm.version", "11.13.0"),
    ("rust.version", "1.96.1"),
];

pub fn expected(tool: &str) -> Option<&'static str> {
    let key = match tool {
        "cargo" | "rustc" => "rust.version",
        "node" => "node.version",
        "pnpm" => "pnpm.version",
        _ => return None,
    };
    VERSIONS
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, version)| *version)
}

pub fn run(key: Option<String>, json: bool) -> i32 {
    if json {
        let held: serde_json::Map<String, serde_json::Value> = VERSIONS
            .iter()
            .map(|(name, version)| (name.to_string(), (*version).into()))
            .collect();
        println!("{}", serde_json::Value::Object(held));
        return 0;
    }
    let known = VERSIONS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ");
    let Some(key) = key else {
        eprintln!("plumb metadata: name one key ({known}) or pass --json");
        return 2;
    };
    match VERSIONS.iter().find(|(name, _)| *name == key) {
        Some((_, version)) => {
            println!("{version}");
            0
        }
        None => {
            eprintln!("plumb metadata: unknown key {key}; known keys are {known}");
            2
        }
    }
}
