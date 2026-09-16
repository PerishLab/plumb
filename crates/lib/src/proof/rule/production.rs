use super::Probe;
use crate::config::{Contract, Execution, Tool};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Production {
    pub platform: String,
    pub target: String,
    pub implementation: String,
    pub environment: Contract,
    pub probes: BTreeMap<String, Vec<Probe>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: String,
    pub contract: String,
    pub platform: String,
    pub artifact: String,
    pub environment: String,
    pub tools: BTreeMap<String, Tool>,
    pub observations: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub commit: String,
    pub tree: String,
}

impl Source {
    pub fn verify(&self) -> Result<(), String> {
        for value in [&self.commit, &self.tree] {
            if value.len() != 40
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(
                    "source provenance requires exact Git commit and tree identities".into(),
                );
            }
        }
        Ok(())
    }
}

pub struct Producer {
    execution: Execution,
    receipt: Receipt,
}

impl Production {
    fn selected(&self) -> Result<BTreeMap<&str, &Probe>, String> {
        if self.platform.is_empty() || self.target.is_empty() || self.implementation.is_empty() {
            return Err(
                "production requires platform, target, and implementation identities".into(),
            );
        }
        self.environment.capture(std::iter::empty())?;
        if self.probes.is_empty() || self.probes.keys().any(|name| name.is_empty()) {
            return Err("production requires named probe rules".into());
        }
        let selected = self
            .probes
            .iter()
            .map(|(name, rules)| Ok((name.as_str(), Probe::select(rules, &self.platform)?)))
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        for binding in self.environment.bind.values() {
            if let crate::config::Binding::Tool { tool } = binding
                && !selected.values().any(|probe| probe.argv[0] == *tool)
            {
                return Err(format!("production requires a probe for bound tool {tool}"));
            }
        }
        Ok(selected)
    }

    pub fn digest(&self) -> Result<String, String> {
        let probes = self.selected()?;
        let body = serde_json::to_vec(&(
            "plumb.production/v1",
            &self.platform,
            &self.target,
            &self.implementation,
            &self.environment,
            probes,
        ))
        .map_err(|error| format!("cannot encode production contract: {error}"))?;
        Ok(format!("{:x}", Sha256::digest(body)))
    }

    pub fn start(&self, root: &Path) -> Result<Producer, String> {
        let programs = self
            .selected()?
            .values()
            .map(|probe| probe.argv[0].clone())
            .collect::<Vec<_>>();
        let execution = Execution::new(
            crate::config::environment(&self.environment)?,
            &programs,
            root,
        )?;
        let receipt = self.observe(&execution)?;
        Ok(Producer { execution, receipt })
    }

    fn observe(&self, execution: &Execution) -> Result<Receipt, String> {
        if self.platform != crate::config::platform() {
            return Err("production platform differs from the executing host".into());
        }
        let mut observations = BTreeMap::new();
        for (name, probe) in self.selected()? {
            let observation = probe.run(execution)?;
            if !observation.matches {
                return Err(format!(
                    "production probe {name} {:?} expected stdout {:?}, observed {:?}",
                    probe.argv, probe.stdout, observation.stdout
                ));
            }
            observations.insert(name.to_string(), observation.stdout);
        }
        Ok(Receipt {
            schema: "plumb.production-receipt/v1".into(),
            contract: self.digest()?,
            platform: self.platform.clone(),
            artifact: String::new(),
            environment: execution.imprint()?,
            tools: execution.tools()?,
            observations,
            source: None,
        })
    }

    pub fn verify(&self, receipt: &Receipt) -> Result<(), String> {
        if let Some(source) = &receipt.source {
            source.verify()?;
        }
        if receipt.schema != "plumb.production-receipt/v1"
            || receipt.contract != self.digest()?
            || receipt.platform != self.platform
            || !hash(&receipt.artifact)
            || !hash(&receipt.environment)
        {
            return Err("production receipt does not bind this contract and artifact".into());
        }
        let selected = self.selected()?;
        if receipt.observations.len() != selected.len() {
            return Err("production receipt has a different probe set".into());
        }
        for (name, probe) in selected {
            if !receipt
                .observations
                .get(name)
                .is_some_and(|text| probe.accepts(text))
            {
                return Err(format!(
                    "production receipt has no matching observation for {name}"
                ));
            }
            if !receipt
                .tools
                .get(&probe.argv[0])
                .is_some_and(|tool| !tool.path.as_os_str().is_empty() && hash(&tool.digest))
            {
                return Err(format!("production receipt has no tool binding for {name}"));
            }
        }
        Ok(())
    }
}

impl Producer {
    pub fn execution(&self) -> &Execution {
        &self.execution
    }

    pub fn finish(mut self, artifact: &Path) -> Result<Receipt, String> {
        if self.receipt.tools != self.execution.tools()? {
            return Err("production tools changed after observation".into());
        }
        self.receipt.artifact = crate::runtime::execution::fingerprint(artifact)?;
        Ok(self.receipt)
    }
}

impl Receipt {
    pub fn verify(&self, artifact: &Path) -> Result<(), String> {
        if self.artifact != crate::runtime::execution::fingerprint(artifact)? {
            return Err("production artifact differs from its receipt".into());
        }
        Ok(())
    }
}

fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
