use std::path::Path;

fn text(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn cold() {
    let held = text(".forgejo/workflows/ship.yml");
    let first = held.split_once("\n  produce:\n").unwrap().0;
    assert!(first.contains("Cold plan without Plumb"));
    assert!(first.contains("dispatch.py --control .plumb-atom"));
    assert!(!first.contains("bootstrap-plumb"));
    assert!(!first.contains("cargo build"));
    assert!(held.contains("PLUMB_SHIP_DECLARATION: ${{ inputs.declaration }}"));
    assert!(!held.contains("PLUMB_BUILD_CONFIGURATION"));
    assert!(
        held.contains("PLUMB_WORKFLOW_INVENTORY_URL: ${{ secrets.WORKFLOW_INVENTORY_ORIGIN }}")
    );
    assert!(!held.contains("secrets.WORKFLOW_INVENTORY_URL"));
    assert_eq!(
        held.matches(
            "forge@sha256:e4d482921c753e337ba5bd1f5b83281451b07982856603d148daea86a0197bea"
        )
        .count(),
        7
    );
}

#[test]
fn matrix() {
    let held = text(".forgejo/workflows/ship.yml");
    for job in ["produce", "bind", "publish", "complete"] {
        assert!(held.contains(&format!("\n  {job}:\n")));
        assert!(held.contains(&format!("fromJSON(needs.resolve.outputs.{job})")));
        assert!(held.contains(&format!("needs.resolve.outputs.{job}_needed == 'true'")));
        assert!(held.contains(&format!("--job {job} --node")));
    }
    assert_eq!(held.matches("runs-on: ${{ matrix.runner }}").count(), 4);
    assert!(held.contains("needs.produce.result == 'skipped'"));
    assert!(held.contains("needs.bind.result == 'skipped'"));
    assert!(held.contains("runner.os == 'Windows'"));
    assert!(held.contains("runner.os != 'Windows'"));
    assert!(held.contains("persist-credentials: false"));
}

#[test]
fn opaque() {
    let held = text(".forgejo/workflows/ship.yml");
    for leaked in [
        "matrix.medium",
        "plumb ship npm",
        "plumb ship chart",
        "plumb ship oci",
        "plumb ship cargo",
        "plumb ship cfworker",
        "plumb workflow plan",
        "bootstrap-plumb",
        "PLUMB_ATOM_HANDOFF",
        "\n  depot:\n",
        "\n  seal:\n",
    ] {
        assert!(!held.contains(leaked), "workflow leaks {leaked}");
    }
}

#[test]
fn preparation() {
    let runner = text(".forgejo/scripts/lib/runner.py");
    assert!(runner.contains("prepared[\"state\"] == \"prepare\""));
    assert!(runner.contains("self.run(dependency, preparation(dependency), preparation)"));
    let controller = text(".forgejo/scripts/controller.py");
    assert!(controller.contains("fingerprint(contract)"));
    assert!(controller.contains("materialized[\"key\"]"));
    assert!(!controller.contains("workflow plan"));
    assert!(!controller.contains("workflow record"));
    let bridge = text(".forgejo/scripts/ship.py");
    assert!(bridge.contains("\"--marker\", configuration[\"marker\"][\"name\"]"));
    assert!(bridge.contains("\"--generation\", configuration[\"generation\"]"));
    assert!(!bridge.contains("PLUMB_BUILD_CONFIGURATION"));
    assert!(bridge.contains("tool = controller(request, seat, environment)"));
}

#[test]
fn interpreter() {
    let workflow = text(".forgejo/workflows/ship.yml");
    assert_eq!(workflow.matches("python.ps1 -Phase probe").count(), 4);
    assert_eq!(workflow.matches("python.ps1 -Phase install").count(), 4);
    assert_eq!(
        workflow.matches("uses: actions/cache/restore@v5").count(),
        4
    );
    assert_eq!(workflow.matches("uses: actions/cache/save@v5").count(), 4);
    assert_eq!(
        workflow.matches("& \"$env:PLUMB_WORKFLOW_PYTHON\"").count(),
        4
    );
    assert!(!workflow.contains("run: python .plumb-atom"));
    let bootstrap = text(".forgejo/scripts/python.ps1");
    assert!(bootstrap.contains("Get-FileHash"));
    assert!(bootstrap.contains("WindowsApps"));
    assert!(!bootstrap.contains("SetEnvironmentVariable"));
    assert!(!bootstrap.contains("GITHUB_PATH"));
}

#[test]
fn marker() {
    let held = text(".forgejo/scripts/fetch-marker.sh");
    assert!(held.contains("bounded_fetch()"));
    assert!(held.contains("kill -TERM \"$fetch_pid\""));
    assert!(held.contains("kill -KILL \"$fetch_pid\""));
    assert!(!held.contains("timeout --"));
}

#[test]
fn singular() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let templates = root.join("assets/ship");
    assert!(
        std::fs::read_dir(&templates)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true)
    );
    for retired in [
        "bootstrap-plumb.sh",
        "bootstrap-plumb.ps1",
        "resolve-ship.sh",
        "execute-ship.sh",
        "execute-ship.ps1",
    ] {
        assert!(!root.join("../../.forgejo/scripts").join(retired).exists());
    }
}
