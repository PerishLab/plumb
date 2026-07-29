use super::model::{Rule, RuleView, Standing};
use super::taxonomy::{self, NamespaceView, OwnerView, TagView};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Subcommand)]
pub enum Deed {
    List {
        #[command(flatten)]
        select: Select,
        #[arg(long)]
        json: bool,
    },
    Show {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Namespaces {
        #[arg(long)]
        json: bool,
    },
    Tags {
        #[arg(long)]
        json: bool,
    },
    Owners {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args, Default)]
pub struct Select {
    #[arg(long)]
    namespace: Option<String>,
    #[arg(
        long = "tag",
        value_delimiter = ',',
        help = "Require every tag; repeat or separate with commas"
    )]
    tags: Vec<String>,
    #[arg(
        long = "any-tag",
        value_delimiter = ',',
        help = "Require at least one tag from this group"
    )]
    any_tags: Vec<String>,
    #[arg(
        long = "without-tag",
        value_delimiter = ',',
        help = "Exclude rules carrying any of these tags"
    )]
    without_tags: Vec<String>,
    #[arg(long = "standing", value_delimiter = ',')]
    standings: Vec<String>,
    #[arg(long = "owner", value_delimiter = ',')]
    owners: Vec<String>,
}

#[derive(Serialize)]
struct ListReport {
    schema: &'static str,
    rules: Vec<RuleView>,
}

#[derive(Serialize)]
struct ShowReport {
    schema: &'static str,
    rule: RuleView,
}

#[derive(Serialize)]
struct NamespaceReport {
    schema: &'static str,
    namespaces: Vec<NamespaceView>,
}

#[derive(Serialize)]
struct TagReport {
    schema: &'static str,
    tags: Vec<TagView>,
}

#[derive(Serialize)]
struct OwnerReport {
    schema: &'static str,
    owners: Vec<OwnerView>,
}

pub fn run(deed: Deed) -> i32 {
    if let Err(error) = super::validate() {
        return sour(&format!("invalid built-in catalog: {error}"));
    }
    match deed {
        Deed::List { select, json } => list(select, json),
        Deed::Show { id, json } => show(&id, json),
        Deed::Namespaces { json } => namespaces(json),
        Deed::Tags { json } => tags(json),
        Deed::Owners { json } => owners(json),
    }
}

fn list(select: Select, json: bool) -> i32 {
    let select = match Selector::new(select) {
        Ok(select) => select,
        Err(error) => return sour(&error),
    };
    let rules = super::all()
        .into_iter()
        .filter(|rule| select.matches(rule))
        .collect::<Vec<_>>();
    if json {
        emit(&ListReport {
            schema: "plumb.rule-list/v1",
            rules: rules.into_iter().map(RuleView::from).collect(),
        });
    } else if rules.is_empty() {
        println!("  no rules match");
    } else {
        for rule in rules {
            println!("  {} [{}] {}", rule.id, rule.standing.id(), rule.summary);
        }
    }
    0
}

fn show(id: &str, json: bool) -> i32 {
    let Some(rule) = super::find(id) else {
        return sour(&format!("unknown rule {id}"));
    };
    if json {
        emit(&ShowReport {
            schema: "plumb.rule/v1",
            rule: RuleView::from(rule),
        });
    } else {
        println!("{}", rule.id);
        println!("  standing  {}", rule.standing.id());
        println!("  owner     {}", rule.owner.id);
        println!(
            "  tags      {}",
            rule.tags
                .iter()
                .map(|tag| tag.id)
                .collect::<Vec<_>>()
                .join(" ")
        );
        println!("  law       {}", rule.law);
        println!("  evidence  {}", rule.evidence);
    }
    0
}

fn namespaces(json: bool) -> i32 {
    let values = taxonomy::NAMESPACES
        .iter()
        .copied()
        .map(NamespaceView::from)
        .collect::<Vec<_>>();
    if json {
        emit(&NamespaceReport {
            schema: "plumb.rule-namespaces/v1",
            namespaces: values,
        });
    } else {
        human(&values, |value| {
            format!("{} [{}] {}", value.id, value.owner, value.summary)
        });
    }
    0
}

fn tags(json: bool) -> i32 {
    let values = taxonomy::TAGS
        .iter()
        .copied()
        .map(TagView::from)
        .collect::<Vec<_>>();
    if json {
        emit(&TagReport {
            schema: "plumb.rule-tags/v1",
            tags: values,
        });
    } else {
        human(&values, |value| format!("{} {}", value.id, value.summary));
    }
    0
}

fn owners(json: bool) -> i32 {
    let values = taxonomy::OWNERS
        .iter()
        .copied()
        .map(OwnerView::from)
        .collect::<Vec<_>>();
    if json {
        emit(&OwnerReport {
            schema: "plumb.rule-owners/v1",
            owners: values,
        });
    } else {
        human(&values, |value| format!("{} {}", value.id, value.summary));
    }
    0
}

fn human<T>(values: &[T], render: impl Fn(&T) -> String) {
    for value in values {
        println!("  {}", render(value));
    }
}

fn emit(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("catalog result should encode")
    );
}

fn sour(note: &str) -> i32 {
    eprintln!("plumb rule: {note}");
    1
}

struct Selector {
    namespace: Option<String>,
    tags: BTreeSet<String>,
    any_tags: BTreeSet<String>,
    without_tags: BTreeSet<String>,
    standings: BTreeSet<String>,
    owners: BTreeSet<String>,
}

impl Selector {
    fn new(raw: Select) -> Result<Self, String> {
        if let Some(namespace) = &raw.namespace {
            known(
                "namespace",
                namespace,
                taxonomy::NAMESPACES.iter().map(|item| item.id),
            )?;
        }
        for tag in raw
            .tags
            .iter()
            .chain(&raw.any_tags)
            .chain(&raw.without_tags)
        {
            known("tag", tag, taxonomy::TAGS.iter().map(|item| item.id))?;
        }
        for owner in &raw.owners {
            known("owner", owner, taxonomy::OWNERS.iter().map(|item| item.id))?;
        }
        for standing in &raw.standings {
            if Standing::parse(standing).is_none() {
                return Err(format!("unknown standing {standing}"));
            }
        }
        Ok(Self {
            namespace: raw.namespace,
            tags: raw.tags.into_iter().collect(),
            any_tags: raw.any_tags.into_iter().collect(),
            without_tags: raw.without_tags.into_iter().collect(),
            standings: raw.standings.into_iter().collect(),
            owners: raw.owners.into_iter().collect(),
        })
    }

    fn matches(&self, rule: &Rule) -> bool {
        let tags = rule.tags.iter().map(|tag| tag.id).collect::<BTreeSet<_>>();
        self.namespace
            .as_deref()
            .is_none_or(|namespace| rule.namespace() == namespace)
            && self.tags.iter().all(|tag| tags.contains(tag.as_str()))
            && (self.any_tags.is_empty()
                || self.any_tags.iter().any(|tag| tags.contains(tag.as_str())))
            && self
                .without_tags
                .iter()
                .all(|tag| !tags.contains(tag.as_str()))
            && (self.standings.is_empty() || self.standings.contains(rule.standing.id()))
            && (self.owners.is_empty() || self.owners.contains(rule.owner.id))
    }
}

fn known<'a>(kind: &str, value: &str, known: impl Iterator<Item = &'a str>) -> Result<(), String> {
    if known.into_iter().any(|item| item == value) {
        Ok(())
    } else {
        Err(format!("unknown {kind} {value}"))
    }
}
