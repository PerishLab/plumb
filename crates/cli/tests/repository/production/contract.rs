use super::{OCI, WORKER, doctor, specimen, write};

const DUAL: &str = "[identity]\nns = \"specimen\"\n[release]\nauthority = \"https://releases.specimen.example\"\nbinaries = [\"specimen-api\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n[release.oci]\nregistry = \"ghcr.io\"\naccount = \"PerishLab\"\n[release.oci.components.api]\nimage = \"perishlab/specimen-api\"\nbinary = \"specimen-api\"\n[release.oci.components.web]\nimage = \"perishlab/specimen-web\"\nweb = \"apps/web\"\nfile = \"deploy/web.Dockerfile\"\n";

const WEB: &str = "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: specimen-web-client\ndata:\n  client.json: \"{}\"\n---\napiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: specimen-web\nspec:\n  template:\n    metadata:\n      annotations:\n        checksum/client: abc123\n    spec:\n      securityContext:\n        fsGroup: 10002\n        fsGroupChangePolicy: OnRootMismatch\n      containers:\n        - name: web\n          volumeMounts:\n            - name: client\n              mountPath: /web/dist/client.json\n              subPath: client.json\n              readOnly: true\n";

#[test]
fn prefix() {
    let root = tempfile::tempdir().unwrap();
    for path in [
        "crates/cli/src",
        "crates/api/src",
        "charts/specimen/templates",
    ] {
        std::fs::create_dir_all(root.path().join(path)).unwrap();
    }
    write(root.path(), ".gitignore", "target/\n");
    write(root.path(), "plumb.toml", "[identity]\nns = \"specimen\"\n");
    write(
        root.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/cli\", \"crates/api\"]\n",
    );
    write(
        root.path(),
        "crates/cli/Cargo.toml",
        "[package]\nname = \"specimen\"\n",
    );
    write(
        root.path(),
        "crates/api/Cargo.toml",
        "[package]\nname = \"api\"\n",
    );
    write(
        root.path(),
        "crates/cli/src/lib.rs",
        "#[plumb::config::prefix]\nstruct Cli;\n",
    );
    write(
        root.path(),
        "crates/api/src/main.rs",
        "#[plumb::config::prefix]\nstruct Api;\nfn main() {}\n",
    );
    let bare = doctor(root.path());
    assert!(
        bare.contains("bare #[plumb::config::prefix] in crates/api/src/main.rs"),
        "{bare}"
    );
    assert!(
        bare.contains("see: plumb cookbook structure.config-prefix-identity"),
        "{bare}"
    );

    write(
        root.path(),
        "crates/api/src/main.rs",
        "#[plumb::config::prefix(\"API\")]\nstruct Api;\nfn main() {}\n",
    );
    let generic = doctor(root.path());
    assert!(
        generic.contains(
            "config prefix \"API\" in crates/api/src/main.rs does not match identity SPECIMEN"
        ),
        "{generic}"
    );

    write(
        root.path(),
        "crates/api/src/main.rs",
        "#[plumb::config::prefix(\"SPECIMEN_API\")]\nstruct Api;\nfn main() {}\n",
    );
    write(
        root.path(),
        "charts/specimen/templates/api.yaml",
        "kind: Deployment\nspec:\n  template:\n    spec:\n      containers:\n        - name: api\n          env:\n            - name: API_LOCUS_KEY\n              value: x\n",
    );
    let env = doctor(root.path());
    assert!(!env.contains("does not match identity"), "{env}");
    assert!(
        env.contains("charts/specimen/templates/api.yaml declares container env API_LOCUS_KEY instead of SPECIMEN_*"),
        "{env}"
    );

    write(
        root.path(),
        "crates/api/src/main.rs",
        "#[plumb::config::prefix(\"SPECIMEN\")]\nstruct Api;\nfn main() {}\n",
    );
    write(
        root.path(),
        "charts/specimen/templates/api.yaml",
        "kind: Deployment\nspec:\n  template:\n    spec:\n      containers:\n        - name: api\n          env:\n            - name: SPECIMEN_LOCUS_KEY\n              value: x\n---\nkind: Secret\nstringData:\n  API_STORE_URL: postgres://x\n  SPECIMEN_STORE_URL: postgres://x\n",
    );
    let clean = doctor(root.path());
    assert!(!clean.contains("config-prefix-identity"), "{clean}");
}

#[test]
fn client() {
    let root = specimen();
    for path in ["apps/web/public", "deploy"] {
        std::fs::create_dir_all(root.path().join(path)).unwrap();
    }
    write(root.path(), "plumb.toml", DUAL);
    write(
        root.path(),
        "apps/web/package.json",
        r#"{"name":"specimen-web","scripts":{"build":"vite build"},"dependencies":{"svelte":"5","vite":"7"}}"#,
    );
    write(root.path(), "Containerfile", "FROM scratch\n");
    write(
        root.path(),
        "deploy/web.Dockerfile",
        "FROM node:22\nCMD [\"node\", \"dist/.perish/server.mjs\"]\n",
    );
    write(
        root.path(),
        "charts/specimen/Chart.yaml",
        "apiVersion: v2\nname: specimen\nversion: 0.0.0\nappVersion: \"0.0.0\"\n",
    );
    write(root.path(), "apps/web/public/client.json", "{}\n");
    write(root.path(), "charts/specimen/templates/web.yaml", WEB);

    let ok = doctor(root.path());
    assert!(!ok.contains("web-client-contract"), "{ok}");

    write(root.path(), "apps/web/public/client.json", "{\"x\":1}\n");
    let dirty = doctor(root.path());
    assert!(
        dirty.contains("empty {} apps/web/public/client.json"),
        "{dirty}"
    );

    write(root.path(), "apps/web/public/client.json", "{}\n");
    write(
        root.path(),
        "charts/specimen/templates/web.yaml",
        &WEB.replace("readOnly: true", "readOnly: false")
            .replace("checksum/client: abc123", "other: abc123"),
    );
    let broken = doctor(root.path());
    assert!(
        broken.contains(
            "volumeMount at /web/dist/client.json with subPath client.json and readOnly: true"
        ),
        "{broken}"
    );
    assert!(
        broken.contains("checksum/client pod template annotation"),
        "{broken}"
    );
    assert!(
        broken.contains("see: plumb cookbook dispatch.web-client-contract"),
        "{broken}"
    );

    write(root.path(), "plumb.toml", &format!("{OCI}{WORKER}"));
    let worker = doctor(root.path());
    assert!(!worker.contains("web-client-contract"), "{worker}");
}

#[test]
fn fsgroup() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("charts/specimen/templates")).unwrap();
    write(
        root.path(),
        "charts/specimen/templates/api.yaml",
        "kind: Deployment\nspec:\n  template:\n    spec:\n      securityContext:\n        fsGroup: 10001\n      containers:\n        - name: api\n          securityContext:\n            readOnlyRootFilesystem: true\n",
    );
    let bad = doctor(root.path());
    assert!(
        bad.contains(
            "charts/specimen/templates/api.yaml sets fsGroup without fsGroupChangePolicy: OnRootMismatch"
        ),
        "{bad}"
    );
    assert!(
        bad.contains("see: plumb cookbook dispatch.chart-fsgroup-on-root-mismatch"),
        "{bad}"
    );

    write(
        root.path(),
        "charts/specimen/templates/api.yaml",
        "kind: Deployment\nspec:\n  template:\n    spec:\n      {{- if .Values.enabled }}\n      securityContext:\n        fsGroup: 10001\n        fsGroupChangePolicy: OnRootMismatch\n      {{- end }}\n      containers:\n        - name: api\n          securityContext:\n            readOnlyRootFilesystem: true\n",
    );
    let ok = doctor(root.path());
    assert!(!ok.contains("chart-fsgroup-on-root-mismatch"), "{ok}");
}

#[test]
fn helpers() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("charts/specimen/templates")).unwrap();
    write(root.path(), "plumb.toml", "[identity]\nns = \"specimen\"\n");
    write(
        root.path(),
        "charts/specimen/templates/_helpers.tpl",
        "{{- define \"specimen.env\" -}}\n- name: API_STORE_KIND\n  value: \"pg\"\n{{- end -}}\n{{- define \"specimen.security\" -}}\n{{- if .Values.enabled }}\nfsGroup: 10001\n{{- end }}\n{{- end -}}\n",
    );
    let bad = doctor(root.path());
    assert!(
        bad.contains(
            "charts/specimen/templates/_helpers.tpl declares container env API_STORE_KIND instead of SPECIMEN_*"
        ),
        "{bad}"
    );
    assert!(
        bad.contains(
            "charts/specimen/templates/_helpers.tpl sets fsGroup without fsGroupChangePolicy: OnRootMismatch"
        ),
        "{bad}"
    );

    write(
        root.path(),
        "charts/specimen/templates/_helpers.tpl",
        "{{- define \"specimen.storekey\" -}}\nAPI_STORE_URL\n{{- end -}}\n{{- define \"specimen.env\" -}}\n- name: SPECIMEN_STORE_KIND\n  value: \"pg\"\n{{- end -}}\n{{- define \"specimen.security\" -}}\n{{- if .Values.enabled }}\nfsGroup: 10001\nfsGroupChangePolicy: OnRootMismatch\n{{- end }}\n{{- end -}}\n",
    );
    let ok = doctor(root.path());
    assert!(!ok.contains("config-prefix-identity"), "{ok}");
    assert!(!ok.contains("chart-fsgroup-on-root-mismatch"), "{ok}");
}

#[test]
fn pullsecrets() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("charts/specimen/templates")).unwrap();
    write(
        root.path(),
        "charts/specimen/values.yaml",
        "replicaCount: 1\n",
    );
    write(
        root.path(),
        "charts/specimen/templates/service.yaml",
        "kind: Service\nmetadata:\n  name: specimen\n",
    );
    write(
        root.path(),
        "charts/specimen/templates/api.yaml",
        "kind: Deployment\nspec:\n  template:\n    spec:\n      containers:\n        - name: api\n          image: nginx:1.29\n",
    );
    let bad = doctor(root.path());
    assert!(
        bad.contains("charts/specimen/values.yaml omits optional imagePullSecrets"),
        "{bad}"
    );
    assert!(
        bad.contains("charts/specimen/templates/api.yaml omits optional imagePullSecrets"),
        "{bad}"
    );
    assert!(
        !bad.contains("service.yaml omits optional imagePullSecrets"),
        "{bad}"
    );
    assert!(
        bad.contains("see: plumb cookbook dispatch.chart-image-pull-secrets"),
        "{bad}"
    );

    write(
        root.path(),
        "charts/specimen/values.yaml",
        "replicaCount: 1\nimagePullSecrets: []\n",
    );
    write(
        root.path(),
        "charts/specimen/templates/api.yaml",
        "kind: Deployment\nspec:\n  template:\n    spec:\n      {{- with .Values.imagePullSecrets }}\n      imagePullSecrets:\n        {{- toYaml . | nindent 8 }}\n      {{- end }}\n      containers:\n        - name: api\n          image: nginx:1.29\n",
    );
    let ok = doctor(root.path());
    assert!(!ok.contains("chart-image-pull-secrets"), "{ok}");
}
