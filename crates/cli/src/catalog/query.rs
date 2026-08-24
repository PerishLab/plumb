use super::model::{Rule, Standing, View};
use super::taxonomy::{self, Label, Ownership, Scope};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Subcommand)]
pub enum Deed {
    #[command(
        about = "List the law, narrowed by namespace, tag, standing, owner, or their absence"
    )]
    List {
        #[command(flatten)]
        select: Select,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Print one rule: its law, its evidence, its standing, and who owns it")]
    Show {
        id: String,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "List the namespaces the catalogue divides itself into")]
    Namespaces {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "List the tags rules are filed under")]
    Tags {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "List the owners a rule may be answerable to")]
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
    any: Vec<String>,
    #[arg(
        long = "without-tag",
        value_delimiter = ',',
        help = "Exclude rules carrying any of these tags"
    )]
    without: Vec<String>,
    #[arg(long = "standing", value_delimiter = ',')]
    standings: Vec<String>,
    #[arg(long = "owner", value_delimiter = ',')]
    owners: Vec<String>,
}

#[derive(Serialize)]
struct List {
    schema: &'static str,
    rules: Vec<View>,
}

#[derive(Serialize)]
struct Detail {
    schema: &'static str,
    rule: View,
}

#[derive(Serialize)]
struct Namespaces {
    schema: &'static str,
    namespaces: Vec<Scope>,
}

#[derive(Serialize)]
struct Tags {
    schema: &'static str,
    tags: Vec<Label>,
}

#[derive(Serialize)]
struct Owners {
    schema: &'static str,
    owners: Vec<Ownership>,
}

pub fn run(deed: Deed) -> i32 {
    if let Err(error) = super::prepare() {
        return sour(&format!("cannot read synced catalog: {error}"));
    }
    if let Err(error) = super::validate() {
        return sour(&format!("invalid synced catalog: {error}"));
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
        emit(&List {
            schema: "plumb.rule-list/v1",
            rules: rules.into_iter().map(View::from).collect(),
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
        emit(&Detail {
            schema: "plumb.rule/v1",
            rule: View::from(rule),
        });
    } else {
        println!("{}", rule.id);
        println!("  standing  {}", rule.standing.id());
        println!("  owner     {}", rule.owner);
        println!("  tags      {}", rule.tags.join(" "));
        println!("  law       {}", rule.law);
        println!("  evidence  {}", rule.evidence);
    }
    0
}

fn namespaces(json: bool) -> i32 {
    let values = taxonomy::namespaces()
        .iter()
        .map(Scope::from)
        .collect::<Vec<_>>();
    if json {
        emit(&Namespaces {
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
    let values = taxonomy::tags().iter().map(Label::from).collect::<Vec<_>>();
    if json {
        emit(&Tags {
            schema: "plumb.rule-tags/v1",
            tags: values,
        });
    } else {
        human(&values, |value| format!("{} {}", value.id, value.summary));
    }
    0
}

fn owners(json: bool) -> i32 {
    let values = taxonomy::owners()
        .iter()
        .map(Ownership::from)
        .collect::<Vec<_>>();
    if json {
        emit(&Owners {
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
    any: BTreeSet<String>,
    without: BTreeSet<String>,
    standings: BTreeSet<String>,
    owners: BTreeSet<String>,
}

impl Selector {
    fn new(raw: Select) -> Result<Self, String> {
        if let Some(namespace) = &raw.namespace {
            known(
                "namespace",
                namespace,
                taxonomy::namespaces().iter().map(|item| item.id.as_str()),
            )?;
        }
        for tag in raw.tags.iter().chain(&raw.any).chain(&raw.without) {
            known(
                "tag",
                tag,
                taxonomy::tags().iter().map(|item| item.id.as_str()),
            )?;
        }
        for owner in &raw.owners {
            known(
                "owner",
                owner,
                taxonomy::owners().iter().map(|item| item.id.as_str()),
            )?;
        }
        for standing in &raw.standings {
            if Standing::parse(standing).is_none() {
                return Err(format!("unknown standing {standing}"));
            }
        }
        Ok(Self {
            namespace: raw.namespace,
            tags: raw.tags.into_iter().collect(),
            any: raw.any.into_iter().collect(),
            without: raw.without.into_iter().collect(),
            standings: raw.standings.into_iter().collect(),
            owners: raw.owners.into_iter().collect(),
        })
    }

    fn matches(&self, rule: &Rule) -> bool {
        let held = rule
            .tags
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let namespace = self
            .namespace
            .as_deref()
            .is_none_or(|namespace| rule.namespace() == namespace);
        let tags = self.tags.iter().all(|tag| held.contains(tag.as_str()));
        let any = self.any.is_empty() || self.any.iter().any(|tag| held.contains(tag.as_str()));
        let without = self.without.iter().all(|tag| !held.contains(tag.as_str()));
        let standing = self.standings.is_empty() || self.standings.contains(rule.standing.id());
        let owner = self.owners.is_empty() || self.owners.contains(rule.owner.as_str());
        [namespace, tags, any, without, standing, owner]
            .into_iter()
            .all(|held| held)
    }
}

fn known<'a>(kind: &str, value: &str, known: impl Iterator<Item = &'a str>) -> Result<(), String> {
    if known.into_iter().any(|item| item == value) {
        Ok(())
    } else {
        Err(format!("unknown {kind} {value}"))
    }
}
