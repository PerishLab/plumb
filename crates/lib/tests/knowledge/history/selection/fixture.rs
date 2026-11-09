pub use super::super::family::fixture::Inputs;
use plumb::semantics::Decision;
use plumb::semantics::admission::{Native, Record, Selection, Selector};
use serde_json::{Value, json};

pub fn setup() -> (Inputs, Selection) {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.present();
    let family = held.inspector().family(&held.reference, "plumb").unwrap();
    let selection = Selection {
        schema: Selection::SCHEMA.into(),
        product: "plumb".into(),
        marker: "v0.138.1".into(),
        root: family.root().clone(),
        family: held.reference.clone(),
        history: held.history.clone(),
        previous: None,
        sequence: 1,
        native: Native {
            run: 3000,
            attempt: 1,
            commit: "a".repeat(40),
            job: 4000,
        },
    };
    issued(&mut held, &selection);
    publish(&mut held, &selection);
    (held, selection)
}

pub fn issued(held: &mut Inputs, selection: &Selection) {
    let reference = selection.reference().unwrap();
    held.fixture
        .values
        .insert(reference.url.clone(), selection.encode().unwrap());
    let base = "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1";
    let mut run: Value = serde_json::from_slice(&held.fixture.values[base]).unwrap();
    run["id"] = selection.native.run.into();
    let route = format!(
        "https://api.github.com/repos/PerishLab/wharf/actions/runs/{}/attempts/1",
        selection.native.run
    );
    held.fixture.put(&route, run);
    let native = &selection.native;
    held.fixture.put(&format!("{route}/jobs?per_page=100"), json!({"total_count":1,"jobs":[{"id":native.job,"run_id":native.run,"run_attempt":1,"head_sha":native.commit,"head_branch":"main","name":"[select] stable family","status":"completed","conclusion":"success","steps":[{"name":format!("[select] family {}",reference.sha256),"status":"completed","conclusion":"success"}]}]}));
}

pub fn publish(held: &mut Inputs, selection: &Selection) {
    let selector = Selector {
        schema: Selector::SCHEMA.into(),
        reference: selection.reference().unwrap(),
    };
    let first = Selector::inception("plumb").unwrap();
    held.fixture
        .values
        .entry(first)
        .or_insert_with(|| selector.encode("plumb").unwrap());
    held.fixture.values.insert(
        Selector::url("plumb").unwrap(),
        selector.encode("plumb").unwrap(),
    );
}

pub fn next(held: &Inputs, previous: &Selection) -> Selection {
    let Record::Transition(record) =
        Record::parse(&held.fixture.values[&held.reference.url]).unwrap()
    else {
        panic!("transition")
    };
    Selection {
        schema: Selection::SCHEMA.into(),
        product: "plumb".into(),
        marker: record.intent.marker,
        root: previous.root.clone(),
        family: held.reference.clone(),
        history: held.history.clone(),
        previous: Some(previous.reference().unwrap()),
        sequence: previous.sequence + 1,
        native: Native {
            run: previous.native.run + 1,
            attempt: 1,
            commit: previous.native.commit.clone(),
            job: previous.native.job + 1,
        },
    }
}

pub struct Initial(pub Inputs);

impl plumb::windtun::qualification::Read for Initial {
    fn bytes(
        &self,
        url: &str,
        limit: usize,
    ) -> Result<Vec<u8>, plumb::windtun::qualification::Fault> {
        self.0.fixture.bytes(url, limit)
    }
    fn optional(
        &self,
        url: &str,
        limit: usize,
    ) -> Result<Option<Vec<u8>>, plumb::windtun::qualification::Fault> {
        if self.0.fixture.values.contains_key(url) {
            self.bytes(url, limit).map(Some)
        } else {
            Ok(None)
        }
    }
    fn image(&self, reference: &str) -> Result<String, plumb::windtun::qualification::Fault> {
        self.0.fixture.image(reference)
    }
}
