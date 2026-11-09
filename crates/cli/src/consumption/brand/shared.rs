use super::{head, receipt};
use std::collections::BTreeMap;

fn image(product: &str, medium: &str) -> receipt::Asset {
    receipt::Asset {
        id: format!("{product}/{medium}"),
        product: product.into(),
        kind: "shared".into(),
        form: "sign".into(),
        step: "full".into(),
        medium: medium.into(),
        surface: "light".into(),
        width: 100,
        height: 100,
        file: format!("media/shared.{medium}"),
        digest: "sha256:synthetic-shared-image".into(),
    }
}

fn outputs() -> BTreeMap<String, Vec<receipt::Asset>> {
    ["svg", "png"]
        .into_iter()
        .map(|medium| {
            (
                format!("/crest/shared.{medium}"),
                vec![image("fixture", medium), image("zeta", medium)],
            )
        })
        .collect()
}

fn document(product: &str) -> String {
    format!(
        r#"<html><head><link rel="icon" href="/crest/shared.svg"><link rel="apple-touch-icon" href="/crest/shared.png"><meta property="og:image" content="https://example.test/crest/shared.png"><meta name="twitter:card" content="summary"><meta name="twitter:image" content="https://example.test/crest/shared.png"></head><body><img data-crest="{product}/png" data-kind="shared" src="/crest/shared.png"></body></html>"#
    )
}

#[test]
fn registered() {
    let outputs = outputs();
    for product in ["fixture", "zeta"] {
        let selection = receipt::Selection {
            product: product.into(),
            origin: "https://example.test".into(),
        };
        assert_eq!(
            head::read(&document(product), &selection, &outputs).unwrap(),
            1
        );
    }
}

#[test]
fn unregistered() {
    let outputs = outputs();
    let selection = receipt::Selection {
        product: "missing".into(),
        origin: "https://example.test".into(),
    };
    let failure = head::read(&document("fixture"), &selection, &outputs).unwrap_err();
    assert!(failure.contains("different formal product"), "{failure}");
    let selection = receipt::Selection {
        product: "fixture".into(),
        origin: "https://example.test".into(),
    };
    let failure = head::read(&document("missing"), &selection, &outputs).unwrap_err();
    assert!(failure.contains("no matching formal output"), "{failure}");
}
