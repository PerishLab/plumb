use std::collections::BTreeMap;
use std::path::Path;

use super::walk;

const TAXONOMY: &str = "[[owner]]\nid = \"fixture\"\nsummary = \"fixture\"\n\n[[tag]]\nid = \"fixture\"\nsummary = \"fixture\"\n";
const POLICY: &str = "[limit]\n\n[comment]\nallow = false\n\n[word]\nsingle = true\n\n[[shape]]\nwhen = [\"fixture-absent\"]\n\n[[web]]\nseat = \"fixture-absent\"\n";
const STRUCTURE: &str = "[dir]\n\n[lane]\n";
const WORKFLOW: &str = r#"[suite]

[execution.cargo]
inherit = ["PATH", "HOME", "USERPROFILE", "SystemRoot", "SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TZ", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH"]
managed = ["CARGO", "CARGO_TARGET_DIR", "CARGO_MANIFEST_*", "CARGO_PKG_*", "CARGO_REGISTRIES_*", "CARGO_BIN_EXE_*", "CARGO_CFG_*", "CARGO_FEATURE_*", "CARGO_PRIMARY_PACKAGE", "CARGO_MAKEFLAGS", "RUST_RECURSION_COUNT", "RUSTUP_TOOLCHAIN_SOURCE"]
reject = ["CARGO_*", "RUST*", "CC", "CC_*", "CXX", "CXX_*", "AR", "AR_*", "CFLAGS", "CFLAGS_*", "CXXFLAGS", "CXXFLAGS_*", "CPPFLAGS", "LDFLAGS", "CMAKE_*", "PKG_CONFIG*", "LD_PRELOAD", "LD_LIBRARY_PATH", "DYLD_*"]

[execution.pnpm]
inherit = ["PATH", "HOME", "USERPROFILE", "SystemRoot", "SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TZ", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH"]
managed = []
reject = []

[execution.probe]
inherit = ["PATH", "HOME", "USERPROFILE", "SystemRoot", "SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TZ", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH", "PLUMB_HOME", "PLUMB_GUARD_CONFIGURATION", "PLUMB_GUARD_DEPOT", "PLUMB_GUARD_VIEW", "PLUMB_DEPOT_SNAPSHOT"]
managed = []
reject = []

[execution.oci]
inherit = ["PATH", "HOME", "USERPROFILE", "SystemRoot", "SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TZ", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH"]
managed = []
reject = []

[execution.registry]
inherit = ["PATH", "HOME", "USERPROFILE", "SystemRoot", "SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TZ", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH"]
managed = []
reject = []
"#;
const DEPS: &str = "blacklist = []\n\n[stable.cargo]\nregistry = \"fixture\"\nindex = \"sparse+https://registry.invalid/\"\n";
const RELEASE: &str =
    "ceiling = 1000\n\n[forge]\nimage = \"fixture\"\n\n[permitted]\n\n[exercised]\n";
const SEAT: &str = "[member]\n";
const VOCABULARY: &str = "schema = 1\ncodec = \"p64-v1\"\nretired = []\n";

pub(super) fn skeleton() -> BTreeMap<String, Vec<u8>> {
    let ids = mechanisms();
    let mut namespaces: Vec<&str> = ids
        .iter()
        .map(|id| id.split('.').next().expect("namespaced mechanism"))
        .collect();
    namespaces.dedup();
    let mut words = TAXONOMY.to_string();
    for namespace in namespaces {
        words.push_str(&format!(
            "\n[[namespace]]\nid = \"{namespace}\"\nsummary = \"fixture\"\nowner = \"fixture\"\n"
        ));
    }
    let mut law = String::new();
    for id in &ids {
        law.push_str(&format!(
            "[[rule]]\nid = \"{id}\"\nsummary = \"fixture\"\nlaw = \"fixture\"\nevidence = \"fixture\"\nstanding = \"mechanized\"\nowner = \"fixture\"\ntags = [\"fixture\"]\n\n"
        ));
    }
    [
        ("rules/taxonomy.toml", words),
        ("rules/catalog.toml", law),
        ("rules/policy.toml", POLICY.to_string()),
        ("rules/structure.toml", STRUCTURE.to_string()),
        ("rules/workflow.toml", WORKFLOW.to_string()),
        ("rules/deps.toml", DEPS.to_string()),
        ("rules/release.toml", RELEASE.to_string()),
        ("rules/seat.toml", SEAT.to_string()),
        ("rules/vocabulary.toml", VOCABULARY.to_string()),
    ]
    .into_iter()
    .map(|(path, text)| (path.to_string(), text.into_bytes()))
    .collect()
}

fn mechanisms() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/catalog/rules");
    let mut ids = Vec::new();
    for file in walk(&root) {
        let text = std::fs::read_to_string(&file).expect("mechanism source");
        for call in text.split("rule!(").skip(1) {
            let args = call.split(')').next().unwrap_or_default();
            if let Some(id) = args.split('"').nth(1) {
                ids.push(id.to_string());
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}
