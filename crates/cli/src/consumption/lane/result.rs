use super::{Reading, plan};
use plumb::lane::Digest;
use plumb::lane::evidence::{Reference, public::Reader};
use plumb::lane::operation::{Authorization, Outcome};

pub(super) fn read(options: Reading) -> Result<serde_json::Value, String> {
    let plan = plan::reviewed(&options.reviewed)?;
    let reference = Reference::new(
        plan.declaration.authorities().publication().clone(),
        Digest::read(&options.evidence)?,
    )?;
    let reader = Reader::new(&options.origin, reference)?;
    let verified = reader.read()?;
    check(&plan, verified.record())?;
    Ok(
        serde_json::json!({"schema":"plumb.lane.result/v1", "plan":plan.digest()?,
        "reference":verified.reference(), "record":verified.record(),
        "current":{"outcome":"unknown","reason":"unverified"}}),
    )
}

fn check(plan: &plan::Plan, record: &plumb::lane::evidence::Record) -> Result<(), String> {
    let operation = record
        .operation()
        .ok_or("lane result requires an operation record")?;
    plan.dispatch.matches(operation.request())?;
    let assessment = operation.assessment();
    if assessment.capabilities() != plan.declaration.capabilities() {
        return Err("lane result capabilities differ from the reviewed declaration".into());
    }
    match assessment.authorization() {
        Authorization::Granted { evidence } | Authorization::Denied { evidence }
            if evidence.authority() != plan.declaration.authorities().authorization() =>
        {
            return Err("lane result authorization authority differs".into());
        }
        _ => {}
    }
    if matches!(operation.outcome(), Outcome::Applied { .. }) {
        plan.declaration
            .admit(operation.request(), assessment)
            .map_err(|_| "lane applied result lacks exact declaration admission")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn fixture() -> (plan::Plan, Value) {
        let target = json!({"repository":"PerishLab/crest","app":"crest","lane":"design"});
        let expected = json!({"state":"present","revision":1,"digest":"1".repeat(64)});
        let declaration = json!({"target":target,"adaptor":"static","capabilities":["inspect","deploy","dispose"],"authorities":{"authorization":"wharf","publication":"wharf"}});
        let dispatch = json!({"schema":"wharf.lane.dispatch/v1","target":target,"operation":"dispose","request":"a".repeat(64),"revision":2,"expected":expected,"source":null,"registration":"e".repeat(64),"caller":"LiberteCode","workflow":format!("PerishLab/wharf/.github/workflows/lane.yml@{}","f".repeat(40))});
        let plan=serde_json::from_value(json!({"schema":"plumb.lane.plan/v1","dispatch":dispatch,"declaration":declaration,"source":{"repository":"PerishLab/crest","commit":"b".repeat(40),"tree":"c".repeat(40)}})).unwrap();
        let request = json!({"context":{"requested":target,"request":"a".repeat(64),"revision":2,"adaptor":"static"},"intent":{"action":"dispose"},"expected":expected});
        let request: plumb::lane::operation::Request = serde_json::from_value(request).unwrap();
        let authorization =
            json!({"state":"granted","evidence":{"authority":"wharf","digest":"d".repeat(64)}});
        let assessment = json!({"request":request.digest().unwrap(),"capabilities":["inspect","deploy","dispose"],"conditions":{"authorization":authorization,"writer":"idle","observed":expected}});
        let operation = json!({"request":request,"assessment":assessment,"outcome":{"outcome":"unknown","reason":"unverified"}});
        let record = json!({"schema":2,"entry":{"kind":"operation","value":operation}});
        (plan, record)
    }

    #[test]
    fn outcomes() {
        let (plan, record) = fixture();
        plan.validate().unwrap();
        for (writer, outcome) in [
            ("idle", json!({"outcome":"unknown","reason":"unverified"})),
            (
                "unknown",
                json!({"outcome":"unknown","reason":"unavailable"}),
            ),
            ("active", json!({"outcome":"refused","reason":"conflict"})),
            (
                "idle",
                json!({"outcome":"applied","material":{"state":"absent"},"writer":"idle"}),
            ),
        ] {
            let mut value = record.clone();
            value["entry"]["value"]["assessment"]["conditions"]["writer"] = json!(writer);
            value["entry"]["value"]["outcome"] = outcome.clone();
            let record = serde_json::from_value(value).unwrap();
            check(&plan, &record).unwrap();
            assert_eq!(
                serde_json::to_value(&record).unwrap()["entry"]["value"]["outcome"],
                outcome
            );
        }
    }

    #[test]
    fn authority() {
        let (plan, record) = fixture();
        let mut changed = record.clone();
        changed["entry"]["value"]["assessment"]["conditions"]["authorization"]["evidence"]["authority"] =
            json!("foreign");
        assert!(check(&plan, &serde_json::from_value(changed).unwrap()).is_err());
        let mut changed = record;
        changed["entry"]["value"]["assessment"]["capabilities"] = json!(["inspect", "dispose"]);
        assert!(check(&plan, &serde_json::from_value(changed).unwrap()).is_err());
    }

    #[test]
    fn confirmation() {
        let (plan, record) = fixture();
        let mut operation = record["entry"]["value"].clone();
        operation["outcome"] =
            json!({"outcome":"applied","material":{"state":"absent"},"writer":"idle"});
        let value = json!({"original":{"authority":"wharf","digest":"a".repeat(64)},"authorization":{"authority":"wharf","digest":"b".repeat(64)},"operation":operation});
        let record = serde_json::from_value(
            json!({"schema":3,"entry":{"kind":"confirmation","value":value}}),
        )
        .unwrap();
        assert_eq!(
            check(&plan, &record).unwrap_err(),
            "lane result requires an operation record"
        );
    }
}
