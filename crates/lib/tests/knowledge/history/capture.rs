use super::*;
use plumb::semantics::admission::{Discovery, Retention};
use plumb::windtun::qualification::{Fault, Read};

const TAGS: &str = "https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1";

struct Source(Fixture);

impl Read for Source {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        self.0.bytes(url, limit)
    }

    fn optional(&self, url: &str, limit: usize) -> Result<Option<Vec<u8>>, Fault> {
        if url == Discovery::url("plumb")? && !self.0.values.contains_key(url) {
            return Ok(None);
        }
        self.0.bytes(url, limit).map(Some)
    }

    fn image(&self, reference: &str) -> Result<String, Fault> {
        self.0.image(reference)
    }
}

struct Inputs {
    source: Source,
    checkpoint: Checkpoint,
    reference: Reference,
    request: Retention,
}

fn inputs(current: bool) -> Inputs {
    let (mut fixture, checkpoint, reference) = setup();
    let tags: Vec<_> = [checkpoint.markers[0].clone(), Tag { marker: "v0.139.0-rc.2".into(), commit: "e".repeat(40) }].iter().map(|tag| json!({"name":tag.marker,"commit":{"sha":tag.commit,"url":format!("https://api.github.com/repos/PerishLab/plumb/commits/{}",tag.commit)}})).collect();
    fixture.put(TAGS, json!(tags));
    if current {
        let head = Discovery {
            schema: Discovery::SCHEMA.into(),
            reference: reference.clone(),
        };
        fixture.values.insert(
            Discovery::url("plumb").unwrap(),
            head.encode("plumb").unwrap(),
        );
    }
    let request = Retention {
        schema: Retention::SCHEMA.into(),
        root: checkpoint.root.clone(),
        previous: current.then(|| reference.clone()),
        native: Native {
            run: 42,
            attempt: 1,
            commit: "a".repeat(40),
            job: 90,
        },
    };
    Inputs {
        source: Source(fixture),
        checkpoint,
        reference,
        request,
    }
}

#[test]
fn bootstrap() {
    let Inputs {
        source,
        checkpoint: original,
        request,
        ..
    } = inputs(false);
    let prepared = Inspector::new(source).retain(&request, "plumb").unwrap();
    assert_eq!(prepared.sequence, 1);
    assert_eq!(prepared.root, original.root);
    assert_eq!(prepared.markers.len(), 2);
    assert!(prepared.previous.is_none());
}

#[test]
fn preservation() {
    let Inputs {
        source,
        checkpoint: original,
        reference,
        request,
    } = inputs(true);
    let prepared = Inspector::new(source).retain(&request, "plumb").unwrap();
    assert_eq!(prepared.sequence, 2);
    assert_eq!(prepared.markers.len(), 3);
    assert!(
        prepared
            .markers
            .iter()
            .any(|tag| tag == &original.markers[1])
    );
    prepared.follows(&reference, &original).unwrap();
}

#[test]
fn retarget() {
    let Inputs {
        mut source,
        checkpoint,
        request,
        ..
    } = inputs(true);
    source.0.edit(TAGS, |tags| {
        tags.as_array_mut().unwrap().push(json!({"name":checkpoint.markers[1].marker,"commit":{"sha":"f".repeat(40),"url":format!("https://api.github.com/repos/PerishLab/plumb/commits/{}", "f".repeat(40))}}));
    });
    let fault = Inspector::new(source)
        .retain(&request, "plumb")
        .unwrap_err();
    assert!(fault.reason.contains("retargets a retained"));
}

#[test]
fn rollback() {
    let Inputs {
        mut source,
        checkpoint,
        reference,
        request,
    } = inputs(true);
    let next = append(&checkpoint, &reference);
    let newer = retained(&mut source.0, &next);
    let head = Discovery {
        schema: Discovery::SCHEMA.into(),
        reference: newer,
    };
    source.0.values.insert(
        Discovery::url("plumb").unwrap(),
        head.encode("plumb").unwrap(),
    );
    let fault = Inspector::new(source)
        .retain(&request, "plumb")
        .unwrap_err();
    assert!(fault.reason.contains("current discovery"));
    let Inputs {
        source,
        mut request,
        ..
    } = inputs(true);
    request.previous = None;
    assert!(Inspector::new(source).retain(&request, "plumb").is_err());
}

#[test]
fn root() {
    let Inputs {
        source,
        reference,
        mut request,
        ..
    } = inputs(true);
    request.root = reference;
    let fault = Inspector::new(source)
        .retain(&request, "plumb")
        .unwrap_err();
    assert!(fault.reason.contains("original adoption"));
}

#[test]
fn current() {
    let Inputs {
        source, reference, ..
    } = inputs(true);
    let history = Inspector::new(source).discover("plumb").unwrap();
    assert_eq!(history.reference(), &reference);
    let report = serde_json::to_value(history).unwrap();
    assert_eq!(report["discovery"], Discovery::url("plumb").unwrap());
    let Inputs { source, .. } = inputs(false);
    assert!(Inspector::new(source).discover("plumb").is_err());
}

#[test]
fn capacity() {
    let Inputs {
        mut source,
        mut checkpoint,
        mut reference,
        mut request,
    } = inputs(true);
    for _ in 1..128 {
        checkpoint = append(&checkpoint, &reference);
        reference = retained(&mut source.0, &checkpoint);
    }
    let head = Discovery {
        schema: Discovery::SCHEMA.into(),
        reference: reference.clone(),
    };
    source.0.values.insert(
        Discovery::url("plumb").unwrap(),
        head.encode("plumb").unwrap(),
    );
    request.previous = Some(reference);
    let fault = Inspector::new(source)
        .retain(&request, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, plumb::windtun::qualification::State::Unknown);
    assert!(fault.reason.contains("capacity"));
}
