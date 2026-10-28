use super::Report;
use super::model::{Edge, Node, Refusal};
use super::read::Graph;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

struct Walk<'a> {
    graph: &'a Graph,
    root: &'a str,
    prior: BTreeMap<String, Edge>,
    pending: VecDeque<String>,
    seen: BTreeSet<String>,
}

struct Trace<'a> {
    graph: &'a Graph,
    adjacent: BTreeMap<String, Vec<usize>>,
    finished: BTreeSet<String>,
    active: BTreeMap<String, usize>,
    stack: Vec<(String, usize)>,
    path: Vec<usize>,
    found: Vec<Vec<Edge>>,
}

impl Graph {
    pub fn refusals(&self, unknown: &mut Vec<String>) -> Vec<Refusal> {
        let mut found = self
            .binaries
            .iter()
            .map(|id| Refusal {
                reason: format!("root workspace declares a binary member: {id}"),
                path: Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut budget = 1000000_usize;
        let mut adjacent = BTreeMap::<&str, Vec<&Edge>>::new();
        for edge in self.edges.iter().filter(|edge| edge.propagated) {
            adjacent.entry(&edge.from).or_default().push(edge);
        }
        for root in &self.roots {
            let mut walk = Walk {
                graph: self,
                root,
                prior: BTreeMap::new(),
                pending: VecDeque::from([root.clone()]),
                seen: BTreeSet::from([root.clone()]),
            };
            if let Err(error) = walk.run(&adjacent, &mut budget, &mut found) {
                unknown.push(error);
                break;
            }
        }
        found
    }

    pub fn cycles(&self, report: &mut Report) -> Vec<Vec<Edge>> {
        let mut trace = Trace {
            graph: self,
            adjacent: BTreeMap::new(),
            finished: BTreeSet::new(),
            active: BTreeMap::new(),
            stack: Vec::new(),
            path: Vec::new(),
            found: Vec::new(),
        };
        for (index, edge) in self
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.propagated)
        {
            let root = report.root == Some(true) && self.roots.contains(&edge.from);
            if edge.from == edge.to && edge.kind == "dev" && !root {
                report.tests.push(edge.clone());
            } else {
                trace
                    .adjacent
                    .entry(project(&self.nodes[&edge.from]))
                    .or_default()
                    .push(index);
            }
        }
        if let Err(error) = trace.run() {
            report.unknown.push(error);
        }
        trace.found
    }
}

impl Walk<'_> {
    fn run(
        &mut self,
        adjacent: &BTreeMap<&str, Vec<&Edge>>,
        budget: &mut usize,
        found: &mut Vec<Refusal>,
    ) -> Result<(), String> {
        while let Some(current) = self.pending.pop_front() {
            for edge in adjacent.get(current.as_str()).into_iter().flatten() {
                permitted(budget, found.len())?;
                found.extend(self.follow(edge)?);
            }
        }
        Ok(())
    }

    fn follow(&mut self, edge: &Edge) -> Result<Option<Refusal>, String> {
        let target = &self.graph.nodes[&edge.to];
        if target.domain || target.workspace {
            return Ok(Some(Refusal {
                reason: format!(
                    "root library {} reaches domain package {}",
                    self.root, target.id
                ),
                path: self.witness(edge)?,
            }));
        }
        if self.seen.insert(edge.to.clone()) {
            self.prior.insert(edge.to.clone(), edge.clone());
            self.pending.push_back(edge.to.clone());
        }
        Ok(None)
    }

    fn witness(&self, edge: &Edge) -> Result<Vec<Edge>, String> {
        let mut path = vec![edge.clone()];
        for _ in 0..4096 {
            let head = path.last().expect("nonempty witness");
            if head.from == self.root {
                path.reverse();
                return Ok(path);
            }
            path.push(
                self.prior
                    .get(&head.from)
                    .ok_or("root witness has no predecessor")?
                    .clone(),
            );
        }
        Err("root witness budget exhausted".into())
    }
}

impl Trace<'_> {
    fn run(&mut self) -> Result<(), String> {
        let starts = self.adjacent.keys().cloned().collect::<Vec<_>>();
        for start in starts {
            if self.finished.contains(&start) {
                continue;
            }
            self.stack.push((start.clone(), 0));
            self.active.insert(start, 0);
            while self.advance()? {}
        }
        Ok(())
    }

    fn advance(&mut self) -> Result<bool, String> {
        let Some((current, cursor)) = self.stack.last_mut() else {
            return Ok(false);
        };
        let next = self
            .adjacent
            .get(current)
            .and_then(|edges| edges.get(*cursor));
        let Some(index) = next.copied() else {
            let (current, _) = self.stack.pop().expect("current frame");
            self.active.remove(&current);
            self.finished.insert(current);
            self.path.pop();
            return Ok(true);
        };
        *cursor += 1;
        self.follow(index)?;
        Ok(true)
    }

    fn follow(&mut self, index: usize) -> Result<(), String> {
        let target = project(&self.graph.nodes[&self.graph.edges[index].to]);
        if let Some(offset) = self.active.get(&target).copied() {
            return self.witness(index, offset);
        }
        if !self.finished.contains(&target) {
            self.active.insert(target.clone(), self.stack.len());
            self.stack.push((target, 0));
            self.path.push(index);
        }
        Ok(())
    }

    fn witness(&mut self, index: usize, offset: usize) -> Result<(), String> {
        if self.found.len() >= 32 || self.path.len() - offset > 4096 {
            return Err("cycle witness budget exhausted".into());
        }
        let mut cycle = self.path[offset..]
            .iter()
            .map(|index| self.graph.edges[*index].clone())
            .collect::<Vec<_>>();
        cycle.push(self.graph.edges[index].clone());
        if cycle.iter().any(|edge| self.graph.nodes[&edge.from].domain) {
            self.found.push(cycle);
        }
        Ok(())
    }
}

fn permitted(budget: &mut usize, witnesses: usize) -> Result<(), String> {
    if *budget == 0 || witnesses >= 256 {
        return Err("root traversal or witness budget exhausted".into());
    }
    *budget -= 1;
    Ok(())
}

fn project(node: &Node) -> String {
    if node.domain {
        format!("perish/{}", node.name)
    } else {
        node.id.clone()
    }
}
