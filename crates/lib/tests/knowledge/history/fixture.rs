use super::super::super::fixture;
use plumb::semantics::admission::{
    Comparison, Inspector, Intent, Native, Record, Reference, Tag, Transition, World,
};
use plumb::semantics::{Baseline, Decision, Release, Request, Review};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub struct Inputs {
    pub fixture: fixture::Fixture,
    pub reference: Reference,
    pub history: Reference,
    count: u64,
}

impl Inputs {
    pub fn new() -> Self {
        let (mut fixture, mut checkpoint, _) = super::super::setup();
        checkpoint.markers.truncate(1);
        let history = super::super::retained(&mut fixture, &checkpoint);
        Self {
            reference: checkpoint.root.clone(),
            fixture,
            history,
            count: 1000,
        }
    }

    pub fn inspector(&self) -> Inspector<fixture::Fixture> {
        let mut fixture = fixture::Fixture::new();
        fixture.values = self.fixture.values.clone();
        Inspector::new(fixture)
    }

    fn object(&mut self, value: &impl serde::Serialize) -> Reference {
        let mut value = serde_json::to_value(value).unwrap();
        value.sort_all_objects();
        let raw = serde_json::to_vec(&value).unwrap();
        let sha256 = format!("{:x}", Sha256::digest(&raw));
        let reference = Reference {
            url: format!("https://releases.plumb.perish.uk/v1/semantics/objects/{sha256}.json"),
            sha256,
            size: raw.len() as u64,
        };
        self.fixture.values.insert(reference.url.clone(), raw);
        reference
    }

    fn issue(&mut self, transition: &mut Transition) {
        self.count += 1;
        transition.intent.native = Native {
            run: self.count,
            attempt: 1,
            commit: "a".repeat(40),
            job: self.count + 100,
        };
        let reference = self.object(&Record::Transition(Box::new(transition.clone())));
        let native = &transition.intent.native;
        let original = "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1";
        let mut run: Value = serde_json::from_slice(&self.fixture.values[original]).unwrap();
        run["id"] = native.run.into();
        let route = format!(
            "https://api.github.com/repos/PerishLab/wharf/actions/runs/{}/attempts/1",
            native.run
        );
        self.fixture.put(&route, run);
        self.fixture.put(&format!("{route}/jobs?per_page=100"), json!({"total_count":1,"jobs":[{"id":native.job,"run_id":native.run,"run_attempt":1,"head_sha":native.commit,"head_branch":"main","name":"[issue] version semantics","status":"completed","conclusion":"success","steps":[{"name":format!("[issue] semantics {}",reference.sha256),"status":"completed","conclusion":"success"}]}]}));
        self.reference = reference;
    }

    fn world(&mut self, marker: &str, original: &World) -> Reference {
        let mut world = original.clone();
        world.source.marker = marker.into();
        let reference = self.object(&world);
        let base = "https://api.github.com/repos/PerishLab/plumb";
        let tag = format!("{:040x}", self.count);
        self.fixture.put(
            &format!("{base}/git/ref/tags/{marker}"),
            json!({"ref":format!("refs/tags/{marker}"),"object":{"type":"tag","sha":tag}}),
        );
        self.fixture.put(
            &format!("{base}/git/tags/{tag}"),
            json!({"sha":tag,"tag":marker,"object":{"type":"commit","sha":world.source.commit}}),
        );
        let route =
            "https://releases.plumb.perish.uk/v1/releases/stable/v0.138.0/distribution.json";
        let mut distribution: Value = serde_json::from_slice(&self.fixture.values[route]).unwrap();
        distribution["marker"] = marker.into();
        distribution["plan"]["marker"] = marker.into();
        let mut combination = serde_json::to_value(world).unwrap();
        combination.as_object_mut().unwrap().remove("guard");
        distribution["resolution"]["binaries"] = combination;
        self.fixture.put(
            &format!(
                "https://releases.plumb.perish.uk/v1/releases/stable/{marker}/distribution.json"
            ),
            distribution,
        );
        reference
    }

    fn review(&mut self, comparison: &Comparison) -> Reference {
        let comment = self.count + 10000;
        let url = format!("https://github.com/PerishLab/.github/issues/178#issuecomment-{comment}");
        let route =
            format!("https://api.github.com/repos/PerishLab/.github/issues/comments/{comment}");
        let body = format!(
            "<!-- plumb.release.review/v2\n{}\n-->",
            serde_json::to_string(comparison).unwrap()
        );
        let mut value: Value =
            serde_json::from_slice(&self.fixture.values[fixture::COMMENT]).unwrap();
        value["id"] = comment.into();
        value["node_id"] = format!("IC_{comment}").into();
        value["html_url"] = url.clone().into();
        value["body"] = body.clone().into();
        self.fixture.put(&route, value);
        Reference {
            url,
            sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
            size: body.len() as u64,
        }
    }

    pub fn append(&mut self, marker: &str, decision: Decision) {
        self.build(marker, decision, false);
    }

    pub fn complete(&mut self, marker: &str, decision: Decision) {
        self.build(marker, decision, true);
    }

    fn build(&mut self, marker: &str, decision: Decision, complete: bool) {
        let family = self.inspector().family(&self.reference, "plumb").unwrap();
        let history = self.inspector().history(&self.history, "plumb").unwrap();
        let mut checkpoint = super::super::append(&history.checkpoints()[0], &self.history);
        if !checkpoint
            .markers
            .iter()
            .any(|tag| tag.marker == family.stable().world().source.marker)
        {
            checkpoint.markers.push(Tag {
                marker: family.stable().world().source.marker.clone(),
                commit: family.stable().snapshot().source.commit.clone(),
            });
        }
        checkpoint.markers.sort_by(|a, b| a.marker.cmp(&b.marker));
        self.history = super::super::retained(&mut self.fixture, &checkpoint);
        let root = self.inspector().record(family.root(), "plumb").unwrap();
        let Record::Adoption(adoption) = root.record else {
            panic!("root")
        };
        let (world, source) = if complete {
            let mut inputs = super::super::super::source::Inputs::new();
            inputs.world.source.marker = marker.into();
            (
                self.world(marker, &inputs.world),
                Some(self.object(&inputs.value())),
            )
        } else {
            (self.world(marker, family.stable().world()), None)
        };
        let candidate = self
            .inspector()
            .formal((&world, source.as_ref()), &adoption.scenes, "plumb")
            .unwrap();
        let comparison = Comparison {
            schema: Comparison::SCHEMA.into(),
            product: "plumb".into(),
            before: family.stable().binding().unwrap(),
            after: candidate.binding().unwrap(),
            anchor: family.original().binding().unwrap(),
            decision,
            basis: "Synthetic exact world-bound judgment; no live publication canary".into(),
        };
        let review = self.review(&comparison);
        let projected = comparison
            .frames(
                &review.url,
                [family.stable(), &candidate, family.original()],
            )
            .unwrap();
        let value = serde_json::to_value(&family).unwrap();
        let continuity = value["continuity"].as_str().map(|location| Review {
            before: projected.anchor.digest().unwrap(),
            after: projected.before.digest().unwrap(),
            anchor: projected.anchor.digest().unwrap(),
            decision: Decision::Same,
            reference: location.into(),
        });
        let request = Request {
            schema: Request::SCHEMA.into(),
            baseline: Baseline {
                stable: Release {
                    marker: family.stable().world().source.marker.clone(),
                    snapshot: projected.before,
                },
                anchor: Release {
                    marker: family.original().world().source.marker.clone(),
                    snapshot: projected.anchor,
                },
                review: continuity,
            },
            candidate: projected.after,
            review: Some(projected.review),
            reservations: checkpoint
                .markers
                .iter()
                .map(|tag| tag.marker.clone())
                .collect(),
            major: None,
        };
        let mut transition = Transition {
            intent: Intent {
                product: "plumb".into(),
                marker: marker.into(),
                root: family.root().clone(),
                previous: self.reference.clone(),
                anchor: family.anchor().clone(),
                request,
                scenes: adoption.scenes,
                world,
                review: Some(review),
                direct: None,
                native: adoption.native,
            },
            history: self.history.clone(),
            source,
        };
        self.issue(&mut transition);
    }

    pub fn present(&mut self) {
        let record = Record::parse(&self.fixture.values[&self.reference.url]).unwrap();
        let Record::Transition(transition) = record else {
            panic!("transition")
        };
        let checkpoint =
            plumb::semantics::admission::Checkpoint::parse(&self.fixture.values[&self.history.url])
                .unwrap();
        let mut checkpoint = super::super::append(&checkpoint, &self.history);
        checkpoint.markers.push(Tag {
            marker: transition.intent.marker.clone(),
            commit: transition.intent.request.candidate.source.commit.clone(),
        });
        checkpoint.markers.sort_by(|a, b| a.marker.cmp(&b.marker));
        self.history = super::super::retained(&mut self.fixture, &checkpoint);
        let reference = self.history.clone();
        self.edit(|transition| transition.history = reference);
        let tags: Vec<_> = checkpoint
            .markers
            .iter()
            .map(|tag| json!({"marker":tag.marker,"commit":tag.commit}))
            .collect();
        let entries: Vec<_> = tags.iter().map(|tag| json!({"name":tag["marker"],"commit":{"sha":tag["commit"],"url":format!("https://api.github.com/repos/PerishLab/plumb/commits/{}",tag["commit"].as_str().unwrap())}})).collect();
        self.fixture.put(
            "https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1",
            json!(entries),
        );
        let head = plumb::semantics::admission::Discovery {
            schema: plumb::semantics::admission::Discovery::SCHEMA.into(),
            reference: self.history.clone(),
        };
        self.fixture.values.insert(
            plumb::semantics::admission::Discovery::url("plumb").unwrap(),
            head.encode("plumb").unwrap(),
        );
    }

    pub fn edit(&mut self, change: impl FnOnce(&mut Transition)) {
        let Record::Transition(mut held) =
            Record::parse(&self.fixture.values[&self.reference.url]).unwrap()
        else {
            panic!("transition")
        };
        change(&mut held);
        self.issue(&mut held);
    }
}
