use super::{Rule, rule};

rule!(
    SIDECAR_MANIFEST_PRESENT,
    "dispatch.sidecar-manifest-present",
    "Web/API pairs declare sidecars",
    "A repository with a web app and executable API declares sidecar.toml.",
    "The repository-root sidecar.toml seat.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    SIDECAR_MANIFEST_READABLE,
    "dispatch.sidecar-manifest-readable",
    "Sidecar manifests are readable",
    "The process dispatch manifest parses as TOML.",
    "Parsing repository-root sidecar.toml.",
    Mechanized,
    SIDECAR,
    [CONFIGURATION, DISPATCH]
);
rule!(
    API_ROLE_DECLARED,
    "dispatch.api-role-declared",
    "Sidecar declares the API role",
    "The sidecar manifest carries a sidecar named api.",
    "The sidecars array in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    API_PORT_LEASED,
    "dispatch.api-port-leased",
    "Sidecar leases the API port",
    "The api sidecar asks the supervisor to lease port zero.",
    "The api sidecar port in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    API_READY_ROLE,
    "dispatch.api-ready-role",
    "API readiness names its role",
    "The api sidecar readiness contract expects role api.",
    "The api readiness table in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    API_HEALTH_ROUTE,
    "dispatch.api-health-route",
    "API health uses the leased port",
    "The api health URL targets {port}/api/health.",
    "The api health_url template in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    WEB_APP_DECLARED,
    "dispatch.web-app-declared",
    "Sidecar declares the web app",
    "The sidecar app is named web.",
    "The app table in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [DISPATCH, WEB_TAG]
);
rule!(
    WEB_PORT_LEASED,
    "dispatch.web-port-leased",
    "Sidecar leases the web port",
    "The web app asks the supervisor to lease port zero.",
    "The web app port in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [DISPATCH, WEB_TAG]
);
rule!(
    WEB_HEALTH_PORT,
    "dispatch.web-health-port",
    "Web health uses the leased port",
    "The web health URL contains the {port} template.",
    "The web app health_url in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [DISPATCH, WEB_TAG]
);
rule!(
    WEB_INHERITS_API_ENDPOINT,
    "dispatch.web-inherits-api-endpoint",
    "Web inherits the API endpoint",
    "The web app inherits api.endpoint under its own API_URL vocabulary.",
    "The web inherits_env entries in sidecar.toml.",
    Mechanized,
    SIDECAR,
    [CONFIGURATION, DISPATCH]
);
rule!(
    API_CONSUMES_PORT,
    "dispatch.api-consumes-port",
    "API consumes its leased port",
    "The API launch or source consumes the sidecar-leased port.",
    "API Rust source and sidecar launch environment.",
    Mechanized,
    SIDECAR,
    [CONFIGURATION, DISPATCH]
);
rule!(
    API_ACCEPTS_STAMP,
    "dispatch.api-accepts-stamp",
    "API accepts the readiness stamp",
    "The API accepts the supervisor's --sidecar-stamp contract.",
    "API Rust source.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    API_EMITS_READINESS,
    "dispatch.api-emits-readiness",
    "API emits endpoint readiness",
    "The API writes readiness carrying role api and its endpoint.",
    "API Rust source.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    API_NAMESPACE_MOUNTED,
    "dispatch.api-namespace-mounted",
    "API mounts the /api namespace",
    "The API application nests its public routes below /api.",
    "API Rust source.",
    Mechanized,
    SIDECAR,
    [DISPATCH]
);
rule!(
    API_IMAGE_PRESENT,
    "dispatch.api-image-present",
    "Production carries an API image",
    "A production web/API repository carries deploy/api.Dockerfile.",
    "The API image build seat.",
    Mechanized,
    RELEASE,
    [DISPATCH, RELEASE_TAG]
);
rule!(
    WEB_IMAGE_PRESENT,
    "dispatch.web-image-present",
    "Production carries a web image",
    "A production web/API repository carries deploy/web.Dockerfile.",
    "The web image build seat.",
    Mechanized,
    RELEASE,
    [RELEASE_TAG, WEB_TAG]
);
rule!(
    WEB_IMAGE_RUNS_DESIGN_RUNTIME,
    "dispatch.web-image-runs-design-runtime",
    "Web image runs the design runtime",
    "The production web image runs the server emitted by the design build.",
    "deploy/web.Dockerfile.",
    Mechanized,
    WEB,
    [DISPATCH, WEB_TAG]
);
rule!(
    WEB_IMAGE_DOES_NOT_OWN_DISPATCH,
    "dispatch.web-image-does-not-own-dispatch",
    "Web image does not proxy API dispatch",
    "The web image leaves public API dispatch to the deployment layer.",
    "Proxy and server directives in deploy/web.Dockerfile.",
    Mechanized,
    WEB,
    [DISPATCH, WEB_TAG]
);
rule!(
    CHART_SPLITS_WORKLOADS,
    "dispatch.chart-splits-workloads",
    "Charts split API and web workloads",
    "Production charts declare distinct api and web workloads.",
    "Workload templates below charts/*/templates.",
    Mechanized,
    RELEASE,
    [DISPATCH, RELEASE_TAG]
);
rule!(
    CHART_SPLITS_INGRESS,
    "dispatch.chart-splits-ingress",
    "Ingress splits API and web routes",
    "Production ingress routes /api to api and / to web.",
    "Ingress templates below charts/*/templates.",
    Mechanized,
    RELEASE,
    [DISPATCH, RELEASE_TAG]
);
rule!(
    CARGO_CHART_VERSION_TRAIN,
    "dispatch.cargo-chart-version-train",
    "Cargo and charts share one version",
    "Cargo, chart version, and chart appVersion move on one release identity.",
    "Cargo.toml and charts/*/Chart.yaml versions.",
    Mechanized,
    RELEASE,
    [CARGO_TAG, RELEASE_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![
        &API_ACCEPTS_STAMP,
        &API_CONSUMES_PORT,
        &API_EMITS_READINESS,
        &API_HEALTH_ROUTE,
        &API_IMAGE_PRESENT,
        &API_NAMESPACE_MOUNTED,
        &API_PORT_LEASED,
        &API_READY_ROLE,
        &API_ROLE_DECLARED,
        &CARGO_CHART_VERSION_TRAIN,
        &CHART_SPLITS_INGRESS,
        &CHART_SPLITS_WORKLOADS,
        &SIDECAR_MANIFEST_PRESENT,
        &SIDECAR_MANIFEST_READABLE,
        &WEB_APP_DECLARED,
        &WEB_HEALTH_PORT,
        &WEB_IMAGE_DOES_NOT_OWN_DISPATCH,
        &WEB_IMAGE_PRESENT,
        &WEB_IMAGE_RUNS_DESIGN_RUNTIME,
        &WEB_INHERITS_API_ENDPOINT,
        &WEB_PORT_LEASED,
    ]
}
