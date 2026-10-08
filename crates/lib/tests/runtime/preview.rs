use plumb::preview::{Observation, Request};
use serde_json::{Value, json};

fn intent() -> Value {
    json!({"schema":"wharf.preview.request/v1","operation":"apply",
        "repository":"PerishLab/crest","app":"crest-review","name":"crest-10","revision":0,
        "request":"c".repeat(32),"registration":"0def022c1a3bb12f7652dec2b08bac0d3c0e1673cb99dae0daf64a8776307bd4",
        "caller":"provider-observed-caller","workflow":"exact-workflow-identity",
        "source":{"commit":"d".repeat(40),"tree":"e".repeat(40),"declaration":"f".repeat(64)}})
}

fn result(request: &Request) -> Value {
    json!({"schema":"wharf.preview.result/v1","request":request.request,"intent":request.digest().unwrap(),
        "outcome":"verified","provider":{"state":"present","quiescent":true,"deployment":{
            "id":"deployment-id","request":request.request,"digest":"1".repeat(64),
            "latest_url":"https://crest-10-crest-review.example.workers.dev",
            "exact_url":"https://deployment-id-crest-review.example.workers.dev"}},
        "content":{"state":"verified","digest":"1".repeat(64)},"failure":null})
}

#[test]
fn requests() {
    Request::read(intent().to_string().as_bytes()).unwrap();
    for (field, value) in [
        ("schema", json!("wharf.preview.request/v2")),
        ("operation", json!("deploy")),
        ("repository", json!("../crest")),
        ("repository", json!("owner/repo/extra")),
        ("repository", json!("owner/r.epo")),
        ("app", json!("1app")),
        ("name", json!("a--b")),
        ("name", json!("a-")),
        ("name", json!("a".repeat(49))),
        ("revision", json!(true)),
        ("revision", json!(-1)),
        ("revision", json!(1.0)),
        ("request", json!("C".repeat(32))),
        ("registration", json!("f".repeat(63))),
        ("caller", json!("")),
        ("caller", json!("é".repeat(257))),
        ("workflow", json!("line\nbreak")),
        ("source", json!(null)),
        ("command", json!("shell")),
    ] {
        let mut held = intent();
        held[field] = value;
        assert!(
            Request::read(held.to_string().as_bytes()).is_err(),
            "{held}"
        );
    }
    for field in intent().as_object().unwrap().keys() {
        let mut held = intent();
        held.as_object_mut().unwrap().remove(field);
        assert!(
            Request::read(held.to_string().as_bytes()).is_err(),
            "{field}"
        );
    }
    let mut held = intent();
    held["revision"] = json!(u64::MAX);
    held["caller"] = json!("é".repeat(256));
    Request::read(held.to_string().as_bytes()).unwrap();
}

#[test]
fn sources() {
    for operation in ["inspect", "discard"] {
        let mut held = intent();
        held["operation"] = json!(operation);
        assert!(Request::read(held.to_string().as_bytes()).is_err());
        held["source"] = Value::Null;
        Request::read(held.to_string().as_bytes()).unwrap();
        held.as_object_mut().unwrap().remove("source");
        assert!(Request::read(held.to_string().as_bytes()).is_err());
    }
    for field in ["commit", "tree", "declaration"] {
        let mut held = intent();
        held["source"].as_object_mut().unwrap().remove(field);
        assert!(Request::read(held.to_string().as_bytes()).is_err());
        held = intent();
        held["source"][field] = json!("bad");
        assert!(Request::read(held.to_string().as_bytes()).is_err());
    }
    for text in [
        intent().to_string().replacen("{", "{\"revision\":0,", 1),
        intent()
            .to_string()
            .replace("\"source\":{", "\"source\":{\"commit\":\"bad\","),
    ] {
        assert!(Request::read(text.as_bytes()).is_err());
    }
}

#[test]
fn canonical() {
    let mut held = intent();
    held["caller"] = json!("评审-é-🧭");
    let request = Request::read(held.to_string().as_bytes()).unwrap();
    let golden = r#"{"app":"crest-review","caller":"评审-é-🧭","name":"crest-10","operation":"apply","registration":"0def022c1a3bb12f7652dec2b08bac0d3c0e1673cb99dae0daf64a8776307bd4","repository":"PerishLab/crest","request":"cccccccccccccccccccccccccccccccc","revision":0,"schema":"wharf.preview.request/v1","source":{"commit":"dddddddddddddddddddddddddddddddddddddddd","declaration":"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","tree":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"},"workflow":"exact-workflow-identity"}"#;
    assert_eq!(request.bytes().unwrap(), golden.as_bytes());
    assert_eq!(
        request.digest().unwrap(),
        "bf6b9f74eaffeba46634df3c1b75d76e5607e0a8655f46ed96f267a961cebd43"
    );
    for (pointer, replacement) in [
        ("/caller", json!("another caller")),
        ("/workflow", json!("another workflow")),
        ("/name", json!("crest-11")),
        ("/app", json!("other-app")),
        ("/repository", json!("PerishLab/design")),
        ("/revision", json!(1)),
        ("/request", json!("9".repeat(32))),
        ("/registration", json!("9".repeat(64))),
        ("/source/commit", json!("9".repeat(40))),
        ("/source/tree", json!("9".repeat(40))),
        ("/source/declaration", json!("9".repeat(64))),
    ] {
        let mut value = serde_json::to_value(&request).unwrap();
        *value.pointer_mut(pointer).unwrap() = replacement;
        let other = Request::read(value.to_string().as_bytes()).unwrap();
        assert_ne!(
            other.digest().unwrap(),
            request.digest().unwrap(),
            "{pointer}"
        );
    }
}

#[test]
fn observations() {
    let request = Request::read(intent().to_string().as_bytes()).unwrap();
    let base = result(&request);
    assert!(
        Observation::read(base.to_string().as_bytes(), &request)
            .unwrap()
            .terminal(&request)
            .unwrap()
    );
    for (pointer, value) in [
        ("/schema", json!("wharf.preview.result/v2")),
        ("/request", json!("9".repeat(32))),
        ("/intent", json!("9".repeat(64))),
        ("/outcome", json!("ready")),
        ("/failure", json!("failed")),
        ("/content/digest", json!("2".repeat(64))),
        ("/provider/deployment/request", json!("2".repeat(32))),
        ("/provider/quiescent", json!(false)),
        ("/provider/state", json!("unknown")),
        ("/provider/deployment", Value::Null),
        ("/content/digest", Value::Null),
    ] {
        let mut held = base.clone();
        *held.pointer_mut(pointer).unwrap() = value;
        assert!(
            Observation::read(held.to_string().as_bytes(), &request).is_err(),
            "{pointer}"
        );
    }
    for pointer in ["", "/provider", "/content", "/provider/deployment"] {
        let keys: Vec<_> = base
            .pointer(pointer)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for key in keys {
            let mut held = base.clone();
            held.pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(&key);
            assert!(
                Observation::read(held.to_string().as_bytes(), &request).is_err(),
                "{pointer}/{key}"
            );
        }
        let mut held = base.clone();
        held.pointer_mut(pointer).unwrap()["extra"] = json!(true);
        assert!(Observation::read(held.to_string().as_bytes(), &request).is_err());
    }
    let repeated = base
        .to_string()
        .replace("\"content\":{", "\"content\":{\"digest\":null,");
    assert!(Observation::read(repeated.as_bytes(), &request).is_err());
}

#[test]
fn uncertainty() {
    let request = Request::read(intent().to_string().as_bytes()).unwrap();
    for outcome in ["unknown", "failed", "degraded"] {
        let mut held = result(&request);
        held["outcome"] = json!(outcome);
        held["content"]["state"] = json!("unverified");
        assert!(Observation::read(held.to_string().as_bytes(), &request).is_err());
        held["failure"] = json!("response lost");
        let observation = Observation::read(held.to_string().as_bytes(), &request).unwrap();
        assert_eq!(
            observation.terminal(&request).unwrap(),
            outcome != "unknown"
        );
        held["provider"]["quiescent"] = json!(false);
        assert_eq!(
            Observation::read(held.to_string().as_bytes(), &request).is_ok(),
            outcome == "unknown"
        );
    }
    let mut held = result(&request);
    held["outcome"] = json!("unknown");
    held["failure"] = json!("provider unavailable");
    held["provider"] = json!({"state":"unknown","quiescent":false,"deployment":null});
    held["content"] = json!({"state":"unknown","digest":null});
    Observation::read(held.to_string().as_bytes(), &request).unwrap();
}

#[test]
fn discarded() {
    let mut value = intent();
    value["operation"] = json!("discard");
    value["source"] = Value::Null;
    let request = Request::read(value.to_string().as_bytes()).unwrap();
    let mut held = result(&request);
    held["outcome"] = json!("discarded");
    held["provider"] = json!({"state":"absent","quiescent":true,"deployment":null});
    held["content"] = json!({"state":"gone","digest":null});
    Observation::read(held.to_string().as_bytes(), &request).unwrap();
    for (pointer, value) in [
        ("/content/state", json!("unknown")),
        ("/content/digest", json!("1".repeat(64))),
        ("/provider/state", json!("unchanged")),
        ("/provider/quiescent", json!(false)),
        ("/failure", json!("not gone")),
    ] {
        let mut other = held.clone();
        *other.pointer_mut(pointer).unwrap() = value;
        assert!(Observation::read(other.to_string().as_bytes(), &request).is_err());
    }
    for pointer in ["/failure", "/provider/deployment", "/content/digest"] {
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let mut other = held.clone();
        other
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(Observation::read(other.to_string().as_bytes(), &request).is_err());
    }
}

#[test]
fn origins() {
    let request = Request::read(intent().to_string().as_bytes()).unwrap();
    for url in [
        "http://a.b.workers.dev",
        "https://example.com",
        "https://a.workers.dev",
        "https://user@a.b.workers.dev",
        "https://a.b.workers.dev:443",
        "https://a.b.workers.dev/x",
        "https://a.b.workers.dev?q=1",
        "https://a.b.workers.dev#x",
        "https://a.b.workers.dev//",
        "https://[",
    ] {
        let mut held = result(&request);
        held["provider"]["deployment"]["latest_url"] = json!(url);
        assert!(
            Observation::read(held.to_string().as_bytes(), &request).is_err(),
            "{url}"
        );
    }
    let mut held = result(&request);
    held["provider"]["deployment"]["exact_url"] =
        held["provider"]["deployment"]["latest_url"].clone();
    assert!(Observation::read(held.to_string().as_bytes(), &request).is_err());
}

#[test]
fn bounded() {
    let request = Request::read(intent().to_string().as_bytes()).unwrap();
    for bytes in [vec![b' '; 65_537], vec![255], b"{".to_vec()] {
        assert!(Request::read(&bytes).is_err());
        assert!(Observation::read(&bytes, &request).is_err());
    }
    let mut invalid = request.clone();
    invalid.schema = "foreign".into();
    assert!(invalid.bytes().is_err());
    assert!(invalid.digest().is_err());
    let observation = Observation::read(result(&request).to_string().as_bytes(), &request).unwrap();
    assert!(observation.terminal(&invalid).is_err());
}
