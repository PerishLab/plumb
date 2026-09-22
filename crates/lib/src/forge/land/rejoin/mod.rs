use super::super::github::{self, Client};
use super::repo::{Repository, Seed, line, success};
use super::{Refusal, refuse};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod stable;

pub use stable::{Stable, latest, tags};

pub const SCHEMA: &str = "plumb.rejoin/v1";
const BASE: &str = "origin/main";
const STATUS: &str = "guard / guard (pull_request)";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub root: PathBuf,
    pub marker: Option<String>,
    pub commit: Option<String>,
    pub state: &'static str,
    pub steps: Vec<String>,
    pub head: Option<String>,
    pub url: Option<String>,
}

struct Owed {
    stable: Stable,
    main: String,
    tree: String,
}

struct Settlement {
    repo: Repository,
}

pub fn plan(root: &Path) -> Result<Report, Refusal> {
    let held = Settlement::open(root)?;
    let Some(owed) = held.owed()? else {
        return held.settled();
    };
    let projection = projection(&owed.stable);
    let steps = vec![
        format!(
            "git commit-tree {} -p {} -p {}",
            owed.tree, owed.main, owed.stable.commit
        ),
        format!("carry the Guard proof {BASE} holds for that tree"),
        format!("git push --force-with-lease origin <rejoin>:refs/heads/{projection}"),
        format!("gh pr create --base main --head {projection} if missing"),
        format!("gh api -X POST repos/<seat>/statuses/<rejoin> (context={STATUS})"),
        "gh pr merge <n> --merge --match-head-commit <rejoin>".to_string(),
        format!("verify {} is an ancestor of {BASE}", owed.stable.marker),
    ];
    Ok(held.report(Some(&owed.stable), "owed", steps))
}

pub fn run(root: &Path) -> Result<Report, Refusal> {
    let held = Settlement::open(root)?;
    let Some(owed) = held.owed()? else {
        return held.settled();
    };
    let head = held.project(&owed)?;
    let projection = projection(&owed.stable);
    held.push(&projection, &head)?;
    let remote = github::remote(&held.repo.root).map_err(|error| refuse("remote", error))?;
    let client = Client::new(&remote);
    let forge = |error| refuse("forge", error);
    let pull = match client.opened("main", &projection).map_err(forge)? {
        Some(pull) => pull,
        None => client
            .raise(
                "main",
                &projection,
                &format!("Rejoin {}", owed.stable.marker),
                &format!(
                    "Records stable {} at {} as merged into main. The tree is main's own, so its Guard proof carries over.",
                    owed.stable.marker, owed.stable.commit
                ),
            )
            .map_err(forge)?,
    };
    client
        .mark(
            &head,
            STATUS,
            "Plumb verified the rejoin keeps main's guarded tree",
        )
        .map_err(forge)?;
    client.settle(pull.number, &head).map_err(forge)?;
    held.repo.fetch()?;
    if !held.ancestor(&owed.stable.commit)? {
        return Err(refuse(
            "unsettled",
            format!(
                "{} merged, yet {BASE} does not hold {}",
                pull.url, owed.stable.marker
            ),
        ));
    }
    let mut report = held.report(Some(&owed.stable), "rejoined", Vec::new());
    report.head = Some(head);
    report.url = Some(pull.url);
    Ok(report)
}

impl Settlement {
    fn open(root: &Path) -> Result<Self, Refusal> {
        let repo = Repository::open(root)?;
        repo.fetch()?;
        Ok(Self { repo })
    }

    fn standing(&self) -> Result<Option<Stable>, Refusal> {
        let listing = self.repo.text(
            &["ls-remote", "--tags", "origin"],
            "remote",
            "cannot list the markers origin holds",
        )?;
        Ok(latest(tags(&listing)))
    }

    fn owed(&self) -> Result<Option<Owed>, Refusal> {
        let Some(stable) = self.standing()? else {
            return Ok(None);
        };
        let reference = format!("refs/tags/{}", stable.marker);
        let output = self
            .repo
            .git(&["fetch", "--no-tags", "origin", &reference])?;
        success(output, "remote", format!("cannot fetch {}", stable.marker))?;
        if self.ancestor(&stable.commit)? {
            return Ok(None);
        }
        let main = self.repo.revision(BASE)?;
        let tree = self.repo.revision(&format!("{main}^{{tree}}"))?;
        if self.merged(&main, &stable)? != tree {
            return Err(refuse(
                "diverged",
                format!(
                    "{} at {} carries changes {BASE} lacks; land them into main first, then rejoin records only the topology",
                    stable.marker, stable.commit
                ),
            ));
        }
        Ok(Some(Owed { stable, main, tree }))
    }

    fn merged(&self, main: &str, stable: &Stable) -> Result<String, Refusal> {
        let output = self.repo.git(&[
            "merge-tree",
            "--write-tree",
            "--no-messages",
            main,
            &stable.commit,
        ])?;
        line(success(
            output,
            "conflict",
            format!(
                "{} conflicts with {BASE}; land the release line's changes into main first",
                stable.marker
            ),
        )?)
    }

    fn project(&self, owed: &Owed) -> Result<String, Refusal> {
        let proof = self.inherited(&owed.main)?;
        let token = proof.encode().map_err(|error| refuse("guard", error))?;
        let message = format!(
            "Rejoin {}\n\nRejoin-Source: {}@{}\n{} {token}\n",
            owed.stable.marker,
            owed.stable.marker,
            owed.stable.commit,
            crate::guard::TRAILER
        );
        let head = self.repo.record(&Seed {
            tree: &owed.tree,
            parents: &[&owed.main, &owed.stable.commit],
            message: &message,
            identity: &BTreeMap::new(),
        })?;
        let carried =
            crate::guard::commit(&self.repo.root, &head).map_err(|error| refuse("guard", error))?;
        if carried != proof {
            return Err(refuse(
                "guard",
                "the rejoin does not carry main's exact Guard proof",
            ));
        }
        Ok(head)
    }

    fn inherited(&self, main: &str) -> Result<crate::guard::Descriptor, Refusal> {
        let guard = |commit: &str| crate::guard::commit(&self.repo.root, commit);
        if let Ok(proof) = guard(main) {
            return Ok(proof);
        }
        let landed = format!("{main}^2");
        let tree = |commit: &str| self.repo.revision(&format!("{commit}^{{tree}}"));
        if self.repo.verified(&landed) && tree(&landed)? == tree(main)? {
            return guard(&landed).map_err(|error| refuse("guard", error));
        }
        Err(refuse(
            "guard",
            format!(
                "{BASE} at {main} carries no Guard proof for its tree, nor does the projection it merged"
            ),
        ))
    }

    fn push(&self, projection: &str, head: &str) -> Result<(), Refusal> {
        let reference = format!("refs/heads/{projection}");
        let tracking = format!("refs/remotes/origin/{projection}");
        let expected = if self.repo.verified(&tracking) {
            self.repo.revision(&tracking)?
        } else {
            String::new()
        };
        let lease = format!("--force-with-lease={reference}:{expected}");
        let spec = format!("{head}:{reference}");
        let output = self.repo.git(&["push", &lease, "origin", &spec])?;
        success(output, "push", format!("cannot push {projection}")).map(|_| ())
    }

    fn ancestor(&self, commit: &str) -> Result<bool, Refusal> {
        let output = self
            .repo
            .git(&["merge-base", "--is-ancestor", commit, BASE])?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => success(
                output,
                "git",
                format!("cannot compare {commit} with {BASE}"),
            )
            .map(|_| false),
        }
    }

    fn settled(&self) -> Result<Report, Refusal> {
        let stable = self.standing()?;
        let state = if stable.is_some() { "home" } else { "unmarked" };
        Ok(self.report(stable.as_ref(), state, Vec::new()))
    }

    fn report(&self, stable: Option<&Stable>, state: &'static str, steps: Vec<String>) -> Report {
        Report {
            schema: SCHEMA,
            root: self.repo.root.clone(),
            marker: stable.map(|held| held.marker.clone()),
            commit: stable.map(|held| held.commit.clone()),
            state,
            steps,
            head: None,
            url: None,
        }
    }
}

fn projection(stable: &Stable) -> String {
    format!("rejoin/{}", stable.marker)
}
