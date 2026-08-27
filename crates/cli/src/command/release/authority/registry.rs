#[path = "profile.rs"]
mod profile;

use plumb::forgejo::{Client, scopes};
use serde::Serialize;

use profile::{Action, Escrow, Model, Observation, Seat, Status};

pub use profile::Input;
const SECRET: &str = "RELEASE_REGISTRY_TOKEN";

struct Context {
    model: Model,
    account: String,
    forge: Client,
}

impl Context {
    fn open(model: Model) -> Result<Self, String> {
        let forge = Client::new(model.remote.clone())?;
        let account = forge.account()?;
        let context = Self {
            model,
            account,
            forge,
        };
        Ok(context)
    }

    fn observe(&self) -> Result<Observation, String> {
        let mut tokens = self
            .forge
            .tokens(&self.account)?
            .into_iter()
            .filter(|token| token.name == self.model.token)
            .collect::<Vec<_>>();
        tokens.sort_by_key(|token| token.id);
        let seat = Seat::new(&self.model.escrow);
        let escrow = seat.load()?;
        let exact = match (tokens.as_slice(), escrow.as_ref()) {
            ([token], Some(held)) if held.identifies(token) && token.scopes == scopes() => {
                self.forge.verify(&self.account, &held.secret).is_ok()
            }
            _ => false,
        };
        Ok(Observation {
            tokens,
            escrow: escrow.as_ref().map(Escrow::view),
            exact,
            secrets: self.forge.secrets()?,
            binding: seat.binding().load()?,
        })
    }

    fn apply(&self, action: Action) -> Result<(), String> {
        match action {
            Action::Capability => self.capability(),
            Action::Repository => self.repository(),
        }
    }

    fn capability(&self) -> Result<(), String> {
        let seat = Seat::new(&self.model.escrow);
        let tokens = self
            .forge
            .tokens(&self.account)?
            .into_iter()
            .filter(|token| token.name == self.model.token)
            .collect::<Vec<_>>();
        for token in &tokens {
            self.forge.revoke(&self.account, token)?;
        }
        seat.retire()?;
        let minted = self.forge.mint(&self.account, &self.model.token)?;
        let token = minted.token.clone();
        let held = Escrow {
            id: token.id,
            name: token.name.clone(),
            last: token.last.clone(),
            secret: minted.secret,
        };
        if let Err(error) = seat.write(&held) {
            return match self.forge.revoke(&self.account, &token) {
                Ok(()) => Err(error),
                Err(revoke) => Err(format!(
                    "{error}; registry capability rollback also failed: {revoke}"
                )),
            };
        }
        Ok(())
    }

    fn repository(&self) -> Result<(), String> {
        let seat = Seat::new(&self.model.escrow);
        let held = seat
            .load()?
            .ok_or_else(|| "registry escrow disappeared after planning".to_string())?;
        let tokens = self
            .forge
            .tokens(&self.account)?
            .into_iter()
            .filter(|token| token.name == self.model.token)
            .collect::<Vec<_>>();
        let [token] = tokens.as_slice() else {
            return Err("registry capability changed after planning".into());
        };
        if !held.identifies(token) || token.scopes != scopes() {
            return Err("registry capability changed after planning".into());
        }
        self.forge.verify(&self.account, &held.secret)?;
        self.forge.set(SECRET, &held.credential())?;
        if !self.forge.secrets()?.contains(SECRET) {
            return Err("Forgejo registry-secret upsert did not verify".into());
        }
        seat.binding().write(held.id)
    }
}

#[derive(Debug, Serialize)]
struct Step {
    resource: &'static str,
    status: Status,
    detail: String,
}

fn build(model: &Model, seen: &Observation) -> (Vec<Step>, Option<Action>) {
    let mut plan = Builder::default();
    if seen.exact {
        plan.ready(
            "registry.capability",
            format!(
                "{} has exact public-only, read:user and write:package scope",
                model.token
            ),
        );
    } else {
        plan.change(
            "registry.capability",
            format!(
                "converge {} and retain its mode-0600 one-time secret",
                model.token
            ),
            Action::Capability,
        );
    }
    let id = seen.escrow.as_ref().map(|held| held.id);
    if seen.exact && seen.binding == id && seen.secrets.contains(SECRET) {
        plan.ready(
            "repository.registry-secret",
            format!("{SECRET} is bound opaquely in {}", model.repository()),
        );
    } else if seen.exact {
        plan.change(
            "repository.registry-secret",
            format!(
                "bind {SECRET} in {} and record the token identity",
                model.repository()
            ),
            Action::Repository,
        );
    } else {
        plan.deferred(
            "repository.registry-secret",
            "waiting for registry.capability",
        );
    }
    plan.finish()
}

#[derive(Default)]
struct Builder {
    steps: Vec<Step>,
    action: Option<Action>,
}

impl Builder {
    fn ready(&mut self, resource: &'static str, detail: impl Into<String>) {
        self.push(resource, Status::Ready, detail);
    }

    fn change(&mut self, resource: &'static str, detail: impl Into<String>, action: Action) {
        self.push(resource, Status::Change, detail);
        self.action.get_or_insert(action);
    }

    fn deferred(&mut self, resource: &'static str, detail: impl Into<String>) {
        self.push(resource, Status::Deferred, detail);
    }

    fn push(&mut self, resource: &'static str, status: Status, detail: impl Into<String>) {
        self.steps.push(Step {
            resource,
            status,
            detail: detail.into(),
        });
    }

    fn finish(self) -> (Vec<Step>, Option<Action>) {
        (self.steps, self.action)
    }
}

struct Plan {
    context: Context,
    seen: Observation,
    steps: Vec<Step>,
    action: Option<Action>,
}

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    product: &'a str,
    account: &'a str,
    repository: String,
    token: &'a str,
    state: &'static str,
    steps: &'a [Step],
    next: Option<Action>,
}

pub fn execute(input: Input) -> Result<(), String> {
    let apply = input.apply;
    let json = input.json;
    let model = Model::read(input)?;
    let mut held = Plan::inspect(model)?;
    if !apply {
        return held.print(json);
    }
    loop {
        if held.action.is_none() {
            return held.print(json);
        }
        held.apply()?;
        held = Plan::inspect(held.context.model.clone())?;
    }
}

impl Plan {
    fn inspect(model: Model) -> Result<Self, String> {
        let context = Context::open(model)?;
        let seen = context.observe()?;
        let (steps, action) = build(&context.model, &seen);
        Ok(Self {
            context,
            seen,
            steps,
            action,
        })
    }

    fn print(&self, json: bool) -> Result<(), String> {
        let report = Report {
            schema: "plumb.registry-authority-plan/v1",
            product: &self.context.model.product,
            account: &self.context.account,
            repository: self.context.model.repository(),
            token: &self.context.model.token,
            state: if self.action.is_none() {
                "ready"
            } else {
                "pending"
            },
            steps: &self.steps,
            next: self.action,
        };
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("cannot encode registry authority plan: {error}"))?
            );
        } else {
            println!("registry authority {}: {}", report.product, report.state);
            for step in report.steps {
                let state = match step.status {
                    Status::Ready => "observed",
                    Status::Change => "change",
                    Status::Deferred => "deferred",
                };
                println!("{state} {}\n  {}", step.resource, step.detail);
            }
        }
        Ok(())
    }

    fn apply(&self) -> Result<(), String> {
        let action = self
            .action
            .ok_or_else(|| "registry authority has no pending action".to_string())?;
        let fresh = Self::inspect(self.context.model.clone())?;
        if fresh.context.account != self.context.account
            || fresh.seen != self.seen
            || fresh.action != self.action
        {
            return Err("registry authority plan became stale".into());
        }
        self.context.apply(action)
    }
}
