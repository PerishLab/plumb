use base64::Engine as _;
use serde::Serialize;
use sha2::{Digest as _, Sha256};

pub(super) fn proof(repository: &str, tree: &str) -> String {
    #[derive(Serialize)]
    struct Claim<'a> {
        schema: &'a str,
        repository: &'a str,
        tree: &'a str,
        plumb: &'a str,
        depot: &'a str,
        platform: &'a str,
        actions: &'a [plumb::guard::Action],
    }
    let actions = vec![plumb::guard::Action {
        name: "guard/test".into(),
        input: "0".repeat(64),
        world: "1".repeat(64),
    }];
    let depot = "2".repeat(64);
    let claim = Claim {
        schema: plumb::guard::SCHEMA,
        repository,
        tree,
        plumb: "v0.0.0",
        depot: &depot,
        platform: "test",
        actions: &actions,
    };
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&claim).expect("claim"))
    );
    let held = plumb::guard::Descriptor {
        schema: claim.schema.into(),
        repository: claim.repository.into(),
        tree: tree.into(),
        plumb: claim.plumb.into(),
        depot: depot.clone(),
        platform: claim.platform.into(),
        actions: actions.clone(),
        digest,
    };
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&held).expect("proof"))
}
