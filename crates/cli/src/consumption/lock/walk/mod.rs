use super::form::Form;
use super::model::{Model, Scope};
use super::record::{Finding, Gap};
use super::tree::{Forest, Parsed, Unit};
use std::collections::BTreeMap;

mod call;
mod emit;
mod hold;
mod infer;
mod macros;
mod method;
mod unit;
mod visit;

#[derive(Default)]
pub struct Harvest {
    pub findings: Vec<Finding>,
    pub gaps: Vec<Gap>,
    pub edges: Vec<Edge>,
    pub direct: BTreeMap<String, String>,
}

pub struct Edge {
    pub caller: String,
    pub callee: String,
    pub owner: String,
    pub finding: Option<usize>,
}

#[derive(Clone)]
pub struct Binding {
    pub form: Form,
    pub origin: Option<String>,
}

pub struct Walker<'a> {
    pub model: &'a Model,
    pub parsed: &'a Parsed,
    pub unit: &'a Unit,
    pub package: String,
    pub scope: Scope,
    pub item: String,
    pub cfg: Vec<String>,
    pub context: &'static str,
    pub frames: Vec<BTreeMap<String, Binding>>,
    pub marks: BTreeMap<usize, usize>,
    pub owners: BTreeMap<usize, String>,
    pub harvest: &'a mut Harvest,
}

pub fn walk(model: &Model, forest: &Forest) -> Harvest {
    let mut harvest = Harvest::default();
    for unit in &forest.units {
        let root = model
            .modules
            .get(&unit.module)
            .map(|module| module.root.clone())
            .unwrap_or_default();
        let mut walker = Walker {
            model,
            parsed: &forest.files[unit.file],
            unit,
            package: model.packages.get(&root).cloned().unwrap_or_default(),
            scope: Scope {
                module: unit.module.clone(),
                ..Scope::default()
            },
            item: unit.module.clone(),
            cfg: unit.cfg.clone(),
            context: unit.context,
            frames: Vec::new(),
            marks: BTreeMap::new(),
            owners: BTreeMap::new(),
            harvest: &mut harvest,
        };
        for item in &unit.items {
            walker.visit(item);
        }
    }
    harvest
}

impl Walker<'_> {
    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        self.frames.iter().rev().find_map(|frame| frame.get(name))
    }

    pub fn define(&mut self, name: String, binding: Binding) {
        if self.frames.is_empty() {
            self.frames.push(BTreeMap::new());
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.insert(name, binding);
        }
    }
}
