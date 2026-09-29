use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn unix() {
    let fixture = seat();
    let (_held, script) = script("migration.sh", SH);
    let output = Command::new("sh")
        .arg(script)
        .current_dir(fixture.path())
        .output()
        .expect("migration");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    migrated(fixture.path());
}

#[test]
fn windows() {
    if Command::new("pwsh").arg("--version").output().is_err() {
        return;
    }
    let fixture = seat();
    let (_held, script) = script("migration.ps1", PS1);
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .current_dir(fixture.path())
        .output()
        .expect("migration");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    migrated(fixture.path());
}

fn seat() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("skills/tool")).expect("skill seat");
    for leaf in ["SKILL.md", "PATHS.md", "SCENARIOS.md"] {
        std::fs::write(root.join("skills/tool").join(leaf), "brief\n").expect("brief");
    }
    std::fs::write(
        root.join("plumb.toml"),
        "[[lock]]\nname = \"skill\"\npaths = [\"skills/tool\"]\nversion = \"1.0.0\"\nhash = \"held\"\n\n[skill]\nstrategy = \"brief\"\n\n[release]\nproduct = \"tool\"\n",
    )
    .expect("manifest");
    fixture
}

fn migrated(root: &Path) {
    let text = std::fs::read_to_string(root.join("plumb.toml")).expect("manifest");
    assert!(!text.contains("[[lock]]"), "{text}");
    assert!(!text.contains("[skill]"), "{text}");
    assert!(text.contains("[release]"), "{text}");
    assert_eq!(text.matches("[[document]]").count(), 2, "{text}");
    assert!(text.contains("strategy = \"agent\""), "{text}");
    assert!(text.contains("strategy = \"brief\""), "{text}");
    assert!(text.contains("name = \"tool\""), "{text}");
    assert_eq!(text.matches("seal = \"\"").count(), 4, "{text}");
}

fn script(name: &str, body: &str) -> (tempfile::TempDir, PathBuf) {
    let held = tempfile::tempdir().expect("script seat");
    let path = held.path().join(name);
    std::fs::write(&path, body).expect("migration script");
    (held, path)
}

const SH: &str = r#"#!/bin/sh
set -eu

manifest=${1:-plumb.toml}
test -f "$manifest"
if grep -q '^\[\[document\]\]' "$manifest"; then
  printf '%s\n' "$manifest already declares document bindings" >&2
  exit 1
fi

draft=$(mktemp "${manifest}.migration.XXXXXX")
trap 'rm -f "$draft"' EXIT HUP INT TERM
awk '
/^\[\[lock\]\][[:space:]]*$/ { skip = 1; next }
/^\[skill\][[:space:]]*$/ { skip = 1; next }
/^\[/ { skip = 0 }
!skip { print }
' "$manifest" > "$draft"

printf '%s\n' \
  '' \
  '[[document]]' \
  'strategy = "agent"' \
  'source = [{ path = ".", seal = "" }]' \
  'target-seal = ""' >> "$draft"

if test -d skills; then
  for skill in skills/*; do
    test -d "$skill" || continue
    test -f "$skill/SKILL.md"
    test -f "$skill/PATHS.md"
    test -f "$skill/SCENARIOS.md"
    name=${skill##*/}
    printf '%s\n' \
      '' \
      '[[document]]' \
      'strategy = "brief"' \
      "name = \"$name\"" \
      'source = [{ path = ".", seal = "" }]' \
      'target-seal = ""' >> "$draft"
  done
fi

mv "$draft" "$manifest"
trap - EXIT HUP INT TERM
printf '%s\n' "$manifest now carries unaffirmed document bindings"
"#;

const PS1: &str = r#"param([string]$Manifest = "plumb.toml")

$ErrorActionPreference = "Stop"
if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) {
  throw "$Manifest does not exist"
}
$lines = Get-Content -LiteralPath $Manifest
if ($lines | Where-Object { $_ -match '^\[\[document\]\]\s*$' }) {
  throw "$Manifest already declares document bindings"
}

$kept = [System.Collections.Generic.List[string]]::new()
$skip = $false
foreach ($line in $lines) {
  if ($line -match '^\[\[lock\]\]\s*$' -or $line -match '^\[skill\]\s*$') {
    $skip = $true
    continue
  }
  if ($line -match '^\[') {
    $skip = $false
  }
  if (-not $skip) {
    $kept.Add($line)
  }
}

$kept.Add("")
$kept.Add("[[document]]")
$kept.Add('strategy = "agent"')
$kept.Add('source = [{ path = ".", seal = "" }]')
$kept.Add('target-seal = ""')

if (Test-Path -LiteralPath "skills" -PathType Container) {
  foreach ($skill in Get-ChildItem -LiteralPath "skills" -Directory | Sort-Object Name) {
    foreach ($leaf in @("SKILL.md", "PATHS.md", "SCENARIOS.md")) {
      if (-not (Test-Path -LiteralPath (Join-Path $skill.FullName $leaf) -PathType Leaf)) {
        throw "$($skill.FullName) misses $leaf"
      }
    }
    $kept.Add("")
    $kept.Add("[[document]]")
    $kept.Add('strategy = "brief"')
    $kept.Add("name = `"$($skill.Name)`"")
    $kept.Add('source = [{ path = ".", seal = "" }]')
    $kept.Add('target-seal = ""')
  }
}

$draft = "$Manifest.migration-$PID"
[System.IO.File]::WriteAllLines($draft, $kept, [System.Text.UTF8Encoding]::new($false))
Move-Item -LiteralPath $draft -Destination $Manifest -Force
Write-Output "$Manifest now carries unaffirmed document bindings"
"#;
