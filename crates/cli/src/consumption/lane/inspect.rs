use super::Inspection;
use plumb::lane::evidence::{
    Entry, Observation, Reason, Record, Reference, Verified, public::Reader,
};
use plumb::lane::{Address, Atom, Digest};

pub(super) fn read(options: Inspection) -> Result<serde_json::Value, String> {
    let expected = Expected::new(&options)?;
    let reference = Reference::new(
        Atom::read(&options.authority)?,
        Digest::read(&options.digest)?,
    )?;
    let reader = Reader::new(&options.origin, reference)?;
    let verified = reader.read()?;
    expected.check(verified.record())?;
    Ok(report(&reader, &verified))
}

struct Expected {
    target: Address,
    context: Option<(Digest, u64)>,
}

impl Expected {
    fn new(options: &Inspection) -> Result<Self, String> {
        let context = match (&options.request, options.revision) {
            (None, None) => None,
            (Some(request), Some(revision)) if revision > 0 => {
                Some((Digest::read(request)?, revision))
            }
            _ => return Err("lane evidence needs a request and positive revision together".into()),
        };
        Ok(Self {
            target: options.selection.address(options.repository.clone())?,
            context,
        })
    }

    fn check(&self, record: &Record) -> Result<(), String> {
        let context = match record.entry() {
            Entry::Selection(selection) => selection.context(),
            Entry::Operation(operation) => operation.request().context(),
            Entry::Confirmation(_) => {
                return Err(
                    "lane confirmation needs an explicit public authorization consumer".into(),
                );
            }
        };
        if context.requested() != &self.target
            || self.context.as_ref().is_some_and(|(request, revision)| {
                request != context.request() || *revision != context.revision()
            })
        {
            return Err("lane public evidence context differs from the exact request".into());
        }
        Ok(())
    }
}

fn report(reader: &Reader, verified: &Verified) -> serde_json::Value {
    serde_json::json!({
        "schema": "plumb.lane.inspection/v1", "url": reader.url(),
        "reference": verified.reference(), "record": verified.record(),
        "current": Observation::Unknown { reason: Reason::Unverified }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumption::lane::Selection;
    use serde_json::json;

    fn options() -> Inspection {
        Inspection {
            selection: Selection {
                app: "crest".into(),
                lane: "design".into(),
            },
            repository: "PerishLab/crest".into(),
            authority: "wharf".into(),
            digest: "a".repeat(64),
            origin: "https://evidence.example.com".into(),
            request: Some("b".repeat(64)),
            revision: Some(2),
            json: true,
        }
    }

    fn record() -> Record {
        let requested = json!({"repository":"PerishLab/crest","app":"crest","lane":"design"});
        let context =
            json!({"requested":requested,"request":"b".repeat(64),"revision":2,"adaptor":"binary"});
        let observation = json!({"outcome":"unknown","reason":"unverified"});
        let value = json!({"policy":"exact","context":context,"observation":observation});
        serde_json::from_value(json!({"schema":2,"entry":{"kind":"selection","value":value}}))
            .unwrap()
    }

    #[test]
    fn history() {
        let record = record();
        Expected::new(&options()).unwrap().check(&record).unwrap();
        let reference =
            Reference::new(Atom::read("wharf").unwrap(), record.digest().unwrap()).unwrap();
        let reader = Reader::new("https://evidence.example.com", reference).unwrap();
        let verified = reader.verify(&record.encode().unwrap()).unwrap();
        let report = report(&reader, &verified);
        assert_eq!(report["record"], serde_json::to_value(&record).unwrap());
        assert_eq!(
            report["current"],
            json!({"outcome":"unknown","reason":"unverified"})
        );
        assert_eq!(
            report["record"]["entry"]["value"]["observation"],
            json!({"outcome":"unknown","reason":"unverified"})
        );
    }

    #[test]
    fn observations() {
        for observation in [
            json!({"outcome":"unmet","reason":"unsupported"}),
            json!({"outcome":"unknown","reason":"unavailable"}),
        ] {
            let mut value = serde_json::to_value(record()).unwrap();
            value["entry"]["value"]["observation"] = observation.clone();
            let record: Record = serde_json::from_value(value).unwrap();
            Expected::new(&options()).unwrap().check(&record).unwrap();
            let reference =
                Reference::new(Atom::read("wharf").unwrap(), record.digest().unwrap()).unwrap();
            let reader = Reader::new("https://evidence.example.com", reference).unwrap();
            let verified = reader.verify(&record.encode().unwrap()).unwrap();
            let report = report(&reader, &verified);
            assert_eq!(
                report["record"]["entry"]["value"]["observation"],
                observation
            );
            assert_eq!(report["current"]["outcome"], "unknown");
        }
    }

    #[test]
    fn exact() {
        for field in ["repository", "app", "lane", "request", "revision"] {
            let mut options = options();
            match field {
                "repository" => options.repository = "PerishLab/other".into(),
                "app" => options.selection.app = "other".into(),
                "lane" => options.selection.lane = "review".into(),
                "request" => options.request = Some("c".repeat(64)),
                _ => options.revision = Some(3),
            }
            assert!(
                Expected::new(&options).unwrap().check(&record()).is_err(),
                "{field}"
            );
        }
        let mut options = options();
        options.request = None;
        assert!(Expected::new(&options).is_err());
        options.revision = None;
        Expected::new(&options).unwrap().check(&record()).unwrap();
        options.request = Some("private-provider-value".into());
        options.revision = Some(0);
        assert!(Expected::new(&options).is_err());
    }

    #[test]
    fn confirmation() {
        let options = options();
        let target = json!({"repository":"PerishLab/crest","app":"crest","lane":"design"});
        let context =
            json!({"requested":target,"request":"b".repeat(64),"revision":2,"adaptor":"static"});
        let expected = json!({"state":"present","revision":1,"digest":"1".repeat(64)});
        let request = json!({"context":context,"intent":{"action":"dispose"},"expected":expected});
        let request: plumb::lane::operation::Request = serde_json::from_value(request).unwrap();
        let authorization =
            json!({"state":"granted","evidence":{"authority":"wharf","digest":"a".repeat(64)}});
        let assessment = json!({"request":request.digest().unwrap(),"capabilities":["dispose"],"conditions":{"authorization":authorization,"writer":"idle","observed":expected}});
        let operation = json!({"request":request,"assessment":assessment,"outcome":{"outcome":"applied","material":{"state":"absent"},"writer":"idle"}});
        let value = json!({"original":{"authority":"wharf","digest":"c".repeat(64)},"authorization":{"authority":"wharf","digest":"d".repeat(64)},"operation":operation});
        let record: Record = serde_json::from_value(
            json!({"schema":3,"entry":{"kind":"confirmation","value":value}}),
        )
        .unwrap();
        let error = Expected::new(&options).unwrap().check(&record).unwrap_err();
        assert!(error.contains("explicit public authorization consumer"));
    }
}
