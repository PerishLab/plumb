use super::finding::{Found, Seed, wrong};
use crate::catalog::model::Mechanism;
use crate::catalog::rules::dispatch as rule;
use crate::shape::dispatch::{Code, Evidence, Image, Manifest, Production, Sidecar, chart};

pub fn judge(evidence: Option<&Evidence>, charts: &chart::Evidence) -> Found {
    let mut found = Found::new();
    if let Some(evidence) = evidence {
        manifest(&evidence.manifest, &mut found);
        code(&evidence.code, &mut found);
        production(&evidence.production, &mut found);
    }
    for path in &charts.mismatch {
        found.push(wrong(
            &rule::CHART_FSGROUP_ON_ROOT_MISMATCH,
            format!(
                "{path} sets fsGroup without fsGroupChangePolicy: OnRootMismatch; see: plumb cookbook dispatch.chart-fsgroup-on-root-mismatch"
            ),
        ));
    }
    for path in &charts.secrets {
        found.push(wrong(
            &rule::CHART_IMAGE_PULL_SECRETS,
            format!(
                "{path} omits optional imagePullSecrets; see: plumb cookbook dispatch.chart-image-pull-secrets"
            ),
        ));
    }
    found
}

fn manifest(manifest: &Manifest, found: &mut Found) {
    match manifest {
        Manifest::Absent => found.push(wrong(
            &rule::SIDECAR_MANIFEST_PRESENT,
            "web/api pair has no sidecar.toml",
        )),
        Manifest::Unread(error) => {
            found.push(Seed::blind(&rule::SIDECAR_MANIFEST_READABLE, error.clone()))
        }
        Manifest::Held(sidecar) => held(sidecar, found),
    }
}

fn held(sidecar: &Sidecar, found: &mut Found) {
    match &sidecar.api {
        None => found.push(wrong(
            &rule::API_ROLE_DECLARED,
            "sidecar does not declare the api role",
        )),
        Some(api) => {
            check(
                found,
                api.port,
                &rule::API_PORT_LEASED,
                "sidecar api must lease port 0",
            );
            check(
                found,
                api.ready,
                &rule::API_READY_ROLE,
                "sidecar api must declare ready role api",
            );
            check(
                found,
                api.health,
                &rule::API_HEALTH_ROUTE,
                "sidecar api health_url must target {port}/api/health",
            );
        }
    }
    check(
        found,
        sidecar.app.named,
        &rule::WEB_APP_DECLARED,
        "sidecar does not declare the web app",
    );
    check(
        found,
        sidecar.app.port,
        &rule::WEB_PORT_LEASED,
        "sidecar web must lease port 0",
    );
    check(
        found,
        sidecar.app.health,
        &rule::WEB_HEALTH_PORT,
        "sidecar web health_url must use {port}",
    );
    check(
        found,
        sidecar.app.binding,
        &rule::WEB_INHERITS_API_ENDPOINT,
        "sidecar web must inherit api.endpoint as API_URL",
    );
}

fn code(code: &Code, found: &mut Found) {
    check(
        found,
        code.port,
        &rule::API_CONSUMES_PORT,
        "api does not consume SIDECAR_PORT",
    );
    check(
        found,
        code.stamp,
        &rule::API_ACCEPTS_STAMP,
        "api does not accept --sidecar-stamp",
    );
    check(
        found,
        code.ready,
        &rule::API_EMITS_READINESS,
        "api does not emit api endpoint readiness",
    );
    check(
        found,
        code.namespace,
        &rule::API_NAMESPACE_MOUNTED,
        "api does not mount the /api namespace",
    );
}

fn production(production: &Production, found: &mut Found) {
    check(
        found,
        production.api,
        &rule::API_IMAGE_PRESENT,
        "production has no api image seat",
    );
    match &production.web {
        Image::Absent => found.push(wrong(
            &rule::WEB_IMAGE_PRESENT,
            "production has no web image seat",
        )),
        Image::Unread | Image::Placed => {}
        Image::Held(image) => {
            check(
                found,
                image.runtime,
                &rule::WEB_IMAGE_RUNS_DESIGN_RUNTIME,
                "web image does not run the emitted design runtime",
            );
            check(
                found,
                image.dispatch,
                &rule::WEB_IMAGE_DOES_NOT_OWN_DISPATCH,
                "web image still owns public proxy dispatch",
            );
        }
    }
    let placed = matches!(production.web, Image::Placed);
    check(
        found,
        production.workloads,
        &rule::CHART_SPLITS_WORKLOADS,
        if placed {
            "chart has no api workload beside the worker-placed web"
        } else {
            "chart does not split api and web workloads"
        },
    );
    check(
        found,
        production.ingress,
        &rule::CHART_SPLITS_INGRESS,
        if placed {
            "chart ingress does not route /api to api beside the worker-placed web"
        } else {
            "chart ingress does not split /api and / between api and web"
        },
    );
    check(
        found,
        production.aligned,
        &rule::CARGO_CHART_VERSION_TRAIN,
        "Cargo and chart do not share one version train",
    );
    if let Some(client) = &production.client {
        let mut missing = Vec::new();
        if !client.seed {
            missing.push("empty {} apps/web/public/client.json");
        }
        if !client.map {
            missing.push("chart ConfigMap carrying client.json");
        }
        if !client.mount {
            missing.push(
                "volumeMount at /web/dist/client.json with subPath client.json and readOnly: true",
            );
        }
        if !client.checksum {
            missing.push("checksum/client pod template annotation");
        }
        if !missing.is_empty() {
            found.push(wrong(
                &rule::WEB_CLIENT_CONTRACT,
                format!(
                    "Dual-OCI web /client.json contract missing {}; see: plumb cookbook dispatch.web-client-contract",
                    missing.join(", ")
                ),
            ));
        }
    }
}

fn check(found: &mut Found, valid: bool, rule: &'static Mechanism, evidence: &'static str) {
    if !valid {
        found.push(wrong(rule, evidence));
    }
}
