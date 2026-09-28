use super::{Entry, Error};

#[derive(Clone, Copy)]
enum Section {
    Trigger,
    Solution,
    Evidence,
    Exit,
}

#[derive(Default)]
struct Sections {
    trigger: Option<String>,
    solution: Option<String>,
    evidence: Option<String>,
    exit: Option<String>,
}

impl Entry {
    pub fn parse(source: &str) -> Result<Self, Error> {
        let mut lines = source.lines();
        let code = lines
            .next()
            .and_then(|line| line.strip_prefix("# "))
            .ok_or_else(|| Error::Format("first line must be `# CODE`".to_string()))?;
        let mut section = None;
        let mut sections = Sections::default();
        for line in lines {
            if let Some(name) = line.strip_prefix("## ") {
                section = Some(sections.open(name)?);
            } else if let Some(section) = section {
                sections.push(section, line);
            } else if !line.trim().is_empty() {
                return Err(Error::Format(format!(
                    "entry `{code}` carries text before its first section"
                )));
            }
        }
        sections.entry(code)
    }
}

impl Sections {
    fn open(&mut self, name: &str) -> Result<Section, Error> {
        let section = match name {
            "Trigger" => Section::Trigger,
            "Solution" => Section::Solution,
            "Evidence" => Section::Evidence,
            "EXIT" => Section::Exit,
            _ => return Err(Error::Format(format!("unknown section `{name}`"))),
        };
        let slot = self.slot(section);
        if slot.is_some() {
            return Err(Error::Format(format!("duplicate section `{name}`")));
        }
        *slot = Some(String::new());
        Ok(section)
    }

    fn push(&mut self, section: Section, line: &str) {
        let held = self.slot(section).as_mut().expect("opened section");
        if !held.is_empty() {
            held.push('\n');
        }
        held.push_str(line);
    }

    fn slot(&mut self, section: Section) -> &mut Option<String> {
        match section {
            Section::Trigger => &mut self.trigger,
            Section::Solution => &mut self.solution,
            Section::Evidence => &mut self.evidence,
            Section::Exit => &mut self.exit,
        }
    }

    fn entry(self, code: &str) -> Result<Entry, Error> {
        let trigger = self.trigger.unwrap_or_default();
        let solution = self.solution.unwrap_or_default();
        let mut entry = Entry::new(code, trigger.trim(), solution.trim())?;
        if let Some(evidence) = self.evidence {
            entry = entry.observe(evidence.trim())?;
        }
        if let Some(exit) = self.exit {
            entry = entry.retire(exit.trim())?;
        }
        Ok(entry)
    }
}
