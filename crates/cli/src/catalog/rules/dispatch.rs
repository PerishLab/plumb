use super::{Mechanism, rule};

rule!(
    SIDECAR_MANIFEST_PRESENT,
    "dispatch.sidecar-manifest-present"
);
rule!(
    SIDECAR_MANIFEST_READABLE,
    "dispatch.sidecar-manifest-readable"
);
rule!(API_ROLE_DECLARED, "dispatch.api-role-declared");
rule!(API_PORT_LEASED, "dispatch.api-port-leased");
rule!(API_READY_ROLE, "dispatch.api-ready-role");
rule!(API_HEALTH_ROUTE, "dispatch.api-health-route");
rule!(WEB_APP_DECLARED, "dispatch.web-app-declared");
rule!(WEB_PORT_LEASED, "dispatch.web-port-leased");
rule!(WEB_HEALTH_PORT, "dispatch.web-health-port");
rule!(
    WEB_INHERITS_API_ENDPOINT,
    "dispatch.web-inherits-api-endpoint"
);
rule!(API_CONSUMES_PORT, "dispatch.api-consumes-port");
rule!(API_ACCEPTS_STAMP, "dispatch.api-accepts-stamp");
rule!(API_EMITS_READINESS, "dispatch.api-emits-readiness");
rule!(API_NAMESPACE_MOUNTED, "dispatch.api-namespace-mounted");
rule!(API_IMAGE_PRESENT, "dispatch.api-image-present");
rule!(WEB_IMAGE_PRESENT, "dispatch.web-image-present");
rule!(
    WEB_IMAGE_RUNS_DESIGN_RUNTIME,
    "dispatch.web-image-runs-design-runtime"
);
rule!(
    WEB_IMAGE_DOES_NOT_OWN_DISPATCH,
    "dispatch.web-image-does-not-own-dispatch"
);
rule!(CHART_SPLITS_WORKLOADS, "dispatch.chart-splits-workloads");
rule!(CHART_SPLITS_INGRESS, "dispatch.chart-splits-ingress");
rule!(
    CARGO_CHART_VERSION_TRAIN,
    "dispatch.cargo-chart-version-train"
);

pub fn mechanisms() -> Vec<&'static Mechanism> {
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
