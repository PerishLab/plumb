use super::super::course::Course;
use crate::command::{diagnostic, github};
use runseal::github::{Cause, Client, Dispatch, Error, Invocation};
use std::collections::BTreeMap;
use std::time::Duration;

const OWNER: &str = "PerishLab";
const REPOSITORY: &str = "wharf";
const BRANCH: &str = "main";
const PATIENCE: u32 = 20;
const INTERVAL: Duration = Duration::from_secs(15);

pub struct Launch<'a> {
    pub workflow: &'a str,
    pub fields: &'a [String],
    pub watch: bool,
}

pub fn launch(course: &mut Course, held: Launch<'_>) -> Result<Option<String>, String> {
    let fields = inputs(held.fields)?;
    let plan = format!(
        "dispatch {OWNER}/{REPOSITORY}/{} at {BRANCH} with {}",
        held.workflow,
        held.fields.join(" ")
    );
    let dispatch = Dispatch::new(held.workflow, BRANCH, fields).map_err(diagnostic)?;
    course.step(plan, || {
        let client = github()?;
        let run = client
            .dispatch(OWNER, REPOSITORY, &dispatch)
            .map_err(diagnostic)?
            .value;
        if held.watch && observe(&client, run.id, &run.url)? != "success" {
            return Err(format!("{} did not succeed", run.url));
        }
        Ok(run.url)
    })
}

fn inputs(fields: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut inputs = BTreeMap::new();
    for field in fields {
        let (name, value) = field.split_once('=').ok_or("invalid Wharf input")?;
        if inputs.insert(name.to_owned(), value.to_owned()).is_some() {
            return Err("duplicate Wharf input".into());
        }
    }
    Ok(inputs)
}

pub fn identity(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or_default()
}

pub fn wait(url: &str) -> Result<String, String> {
    let id = identity(url).parse().map_err(|_| "invalid Wharf run ID")?;
    observe(&github()?, id, url)
}

fn observe(client: &Client, id: u64, url: &str) -> Result<String, String> {
    let invocation = Invocation::new(id, None).map_err(diagnostic)?;
    let mut failures = 0;
    let mut said = String::new();
    loop {
        match client.workflow(OWNER, REPOSITORY, invocation) {
            Ok(receipt) if receipt.value.status == "completed" => {
                announce(url, &receipt.value.status, &mut said);
                return receipt
                    .value
                    .conclusion
                    .ok_or(format!("{url}: completed run omitted conclusion"));
            }
            Ok(receipt) => {
                failures = 0;
                announce(url, &receipt.value.status, &mut said);
            }
            Err(error) if failures + 1 == PATIENCE || !retry(&error) => {
                return Err(format!("{url}: {}", diagnostic(error)));
            }
            Err(_) => failures += 1,
        }
        std::thread::sleep(INTERVAL);
    }
}

fn retry(error: &Error) -> bool {
    match error.cause {
        Cause::Network | Cause::Read => true,
        Cause::Http => matches!(error.evidence.status, Some(404 | 500..=599)),
        _ => false,
    }
}

fn announce(url: &str, status: &str, said: &mut String) {
    if status != said {
        eprintln!("{url} {status}");
        *said = status.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::{Launch, inputs, launch};
    use crate::command::operator::course::Course;

    #[test]
    fn plan() {
        let mut course = Course::new(true);
        let fields = [
            "marker=v0.54.0".into(),
            "repository=PerishLab/runseal".into(),
        ];
        let request = Launch {
            workflow: "ship.yml",
            fields: &fields,
            watch: true,
        };
        assert_eq!(launch(&mut course, request).unwrap(), None);
        assert_eq!(
            course.plan(),
            "dispatch PerishLab/wharf/ship.yml at main with marker=v0.54.0 repository=PerishLab/runseal"
        );
    }

    #[test]
    fn fields() {
        let one = "name=with=separator".into();
        assert_eq!(inputs(&[one]).unwrap()["name"], "with=separator");
        let duplicate = ["name=first".into(), "name=second".into()];
        assert!(inputs(&duplicate).is_err());
    }
}
