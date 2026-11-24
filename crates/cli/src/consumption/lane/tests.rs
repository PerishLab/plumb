use super::{Execution, Planning, Reviewed, Selection, dispatch, hub, plan};
use crate::consumption::lane::hub::Gateway;
use plumb::cli::Root;
use plumb::lane::dispatch::Operation;
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;

#[path = "../../../tests/command/lane/fixture.rs"]
mod product;
use product::{fixture, git};

struct Forge {
    login: String,
    head: String,
    commit: String,
    tree: String,
    fault: Cell<bool>,
    loss: Cell<bool>,
    sent: RefCell<Vec<Value>>,
}

impl Gateway for Forge {
    fn observe(&self, plan: &plan::Plan) -> Result<(), String> {
        if self.fault.get() {
            return Err("GitHub observation refused".into());
        }
        hub::binding(plan, &self.login, &self.head)?;
        hub::qualify(plan, &self.commit, &self.tree)
    }

    fn submit(&self, plan: &plan::Plan) -> Result<(), String> {
        let request = hub::payload(plan)?;
        self.sent
            .borrow_mut()
            .push(json!({"ref":"main","inputs":{"request":request}}));
        if self.loss.get() {
            return Err(
                "Wharf dispatch acknowledgment unverified; operation outcome unknown".into(),
            );
        }
        Ok(())
    }
}

struct Seat {
    product: tempfile::TempDir,
    home: tempfile::TempDir,
    forge: Forge,
}

impl Seat {
    fn new() -> Self {
        let product = fixture();
        let forge = Forge {
            login: "LiberteCode".into(),
            head: "f".repeat(40),
            commit: git(product.path(), &["rev-parse", "HEAD"]),
            tree: git(product.path(), &["rev-parse", "HEAD^{tree}"]),
            fault: Cell::new(false),
            loss: Cell::new(false),
            sent: RefCell::new(Vec::new()),
        };
        let home = tempfile::tempdir().unwrap();
        let held = Self {
            product,
            home,
            forge,
        };
        let source =
            json!({"commit":held.forge.commit,"tree":held.forge.tree,"declaration":"d".repeat(64)});
        let target = json!({"repository":"PerishLab/crest","app":"crest","lane":"design"});
        let document = json!({
            "schema":"wharf.lane.dispatch/v1", "target":target, "source":source,
            "operation":"apply", "request":"a".repeat(64), "revision":1,
            "expected":{"state":"absent"}, "registration":"e".repeat(64),
            "caller":"LiberteCode", "workflow":format!(
                "PerishLab/wharf/.github/workflows/lane.yml@{}", held.forge.head
            )
        });
        held.write("input.json", &document);
        held
    }

    fn write(&self, name: &str, value: &Value) {
        std::fs::write(self.path(name), serde_json::to_vec(value).unwrap()).unwrap();
    }

    fn read(&self, name: &str) -> Value {
        serde_json::from_slice(&std::fs::read(self.path(name)).unwrap()).unwrap()
    }

    fn path(&self, name: &str) -> PathBuf {
        self.home.path().join(name)
    }

    fn root(&self) -> Root {
        Root {
            root: self.product.path().to_str().unwrap().into(),
        }
    }

    fn prepare(&self) -> Value {
        let options = Planning {
            target: self.root(),
            selection: Selection {
                app: "crest".into(),
                lane: "design".into(),
            },
            input: self.path("input.json"),
            json: true,
        };
        let result = plan::prepare(options, &self.forge).unwrap();
        self.write("plan.json", &result);
        result
    }

    fn execute(&self, operation: Operation, digest: &str) -> Result<Value, String> {
        let reviewed = Reviewed {
            plan: self.path("plan.json"),
            digest: digest.into(),
        };
        let options = Execution {
            target: self.root(),
            reviewed,
            json: true,
        };
        dispatch::execute(options, operation, &self.forge)
    }
}

#[test]
fn submit() {
    let seat = Seat::new();
    let index = std::fs::read(seat.product.path().join(".git/index")).unwrap();
    let report = seat.prepare();
    assert_eq!(report["current"]["outcome"], "unknown");
    assert!(seat.forge.sent.borrow().is_empty());
    let result = seat
        .execute(Operation::Apply, report["digest"].as_str().unwrap())
        .unwrap();
    assert_eq!(result["submission"], "acknowledged");
    assert_eq!(result["operation"]["outcome"], "unknown");
    assert_eq!(result["current"]["outcome"], "unknown");
    let posts = seat.forge.sent.borrow();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["ref"], "main");
    assert_eq!(
        serde_json::from_str::<Value>(posts[0]["inputs"]["request"].as_str().unwrap()).unwrap(),
        seat.read("input.json")
    );
    assert_eq!(
        std::fs::read(seat.product.path().join(".git/index")).unwrap(),
        index
    );
    assert!(
        !seat
            .product
            .path()
            .join("apps/review/SHOULD_NOT_EXIST")
            .exists()
    );
}

#[test]
fn moved() {
    for key in 0..4 {
        let mut seat = Seat::new();
        let report = seat.prepare();
        let value = match key {
            0 => &mut seat.forge.login,
            1 => &mut seat.forge.head,
            2 => &mut seat.forge.commit,
            _ => &mut seat.forge.tree,
        };
        *value = "moved".into();
        assert!(
            seat.execute(Operation::Apply, report["digest"].as_str().unwrap())
                .is_err()
        );
        assert!(seat.forge.sent.borrow().is_empty());
    }
}

#[test]
fn review() {
    let seat = Seat::new();
    let report = seat.prepare();
    let digest = report["digest"].as_str().unwrap();
    assert!(seat.execute(Operation::Dispose, digest).is_err());
    assert!(seat.execute(Operation::Apply, &"0".repeat(64)).is_err());
    let mut changed = report.clone();
    changed["plan"]["dispatch"]["request"] = json!("b".repeat(64));
    seat.write("plan.json", &changed);
    assert!(seat.execute(Operation::Apply, digest).is_err());
    seat.write("plan.json", &report);
    std::fs::write(seat.product.path().join("changed"), "preserve").unwrap();
    assert!(seat.execute(Operation::Apply, digest).is_err());
    git(seat.product.path(), &["add", "."]);
    git(
        seat.product.path(),
        &["commit", "--quiet", "-m", "source moved"],
    );
    assert!(seat.execute(Operation::Apply, digest).is_err());
    assert!(seat.forge.sent.borrow().is_empty());
}

#[test]
fn uncertain() {
    let seat = Seat::new();
    let report = seat.prepare();
    seat.forge.loss.set(true);
    let error = seat
        .execute(Operation::Apply, report["digest"].as_str().unwrap())
        .unwrap_err();
    assert!(error.contains("outcome unknown"));
    assert_eq!(seat.forge.sent.borrow().len(), 1);
}

#[test]
fn discard() {
    let seat = Seat::new();
    let mut input = seat.read("input.json");
    input["operation"] = json!("dispose");
    input["source"] = Value::Null;
    input["revision"] = json!(2);
    input["expected"] = json!({"state":"present","revision":1,"digest":"1".repeat(64)});
    seat.write("input.json", &input);
    let report = seat.prepare();
    let digest = report["digest"].as_str().unwrap();
    seat.execute(Operation::Dispose, digest).unwrap();
    assert!(seat.execute(Operation::Apply, digest).is_err());
    assert_eq!(seat.forge.sent.borrow().len(), 1);
}

#[test]
fn unavailable() {
    let seat = Seat::new();
    let report = seat.prepare();
    seat.forge.fault.set(true);
    assert!(
        seat.execute(Operation::Apply, report["digest"].as_str().unwrap())
            .is_err()
    );
    assert!(seat.forge.sent.borrow().is_empty());
}
