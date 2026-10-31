use super::{Read, Result, source::Subject, wire::Wire};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) struct Receipt {
    pub index: Value,
    pub intent: Value,
    pub result: Value,
    pub passed: Value,
    pub root: String,
    pub attempt: String,
}

impl Receipt {
    pub fn load<R: Read>(reader: &R, subject: &Subject, policy: &Value) -> Result<Self> {
        let marker = Wire::text(&subject.source["marker"])?;
        let root = format!(
            "{}/v1/releases/{}/{marker}/windtun",
            subject.authority, subject.channel
        );
        let index = Wire::document(reader, &format!("{root}/latest.json"), true)?;
        Wire::fields(&index, &["schema", "observer", "started", "intent"])?;
        Wire::require(
            index["schema"] == "wharf.windtun.index/v1",
            "unsupported qualification discovery",
        )?;
        Wire::observer(&index["observer"])?;
        Self::clock(&index["started"])?;
        let observer = &index["observer"];
        let attempt = format!(
            "{root}/attempts/{}-{}",
            Wire::text(&observer["run"])?,
            Wire::text(&observer["attempt"])?
        );
        Wire::require(
            index["intent"]["url"] == format!("{attempt}/intent.json"),
            "discovery names a different attempt",
        )?;
        let intent = Wire::parse(
            &Wire::read(reader, &index["intent"], &subject.authority)?,
            true,
        )?;
        Wire::fields(
            &intent,
            &[
                "schema",
                "observer",
                "started",
                "source",
                "policy",
                "scope",
                "preparation",
            ],
        )?;
        Wire::matches(
            &intent,
            &json!({"schema":"wharf.windtun.intent/v1", "observer":observer, "started":index["started"], "source":subject.source, "scope":"declared", "policy":policy["policy"]}),
            "intent differs from current declared subject",
        )?;
        Self::material(&intent["preparation"], &["key", "basis", "record"])?;
        let result = Wire::document(reader, &format!("{attempt}/result.json"), true)?;
        Wire::require(
            result["ok"] == true && result["state"] == "passed",
            "latest attempt is not a complete success",
        )?;
        Wire::fields(
            &result,
            &[
                "schema",
                "observer",
                "source",
                "intent",
                "state",
                "ok",
                "credential",
                "evidence",
            ],
        )?;
        Wire::matches(
            &result,
            &json!({"schema":"wharf.windtun.result/v1", "observer":observer, "source":subject.source, "intent":Wire::digest(&intent)?}),
            "result belongs to another attempt or subject",
        )?;
        Wire::require(
            result["credential"]["url"] == format!("{attempt}/pass.json"),
            "result names a different credential",
        )?;
        Self::material(&result["evidence"], &["key", "record", "digest"])?;
        let passed = Wire::parse(
            &Wire::read(reader, &result["credential"], &subject.authority)?,
            true,
        )?;
        let receipt = Self {
            index,
            intent,
            result,
            passed,
            root,
            attempt,
        };
        receipt.validate(subject, policy)?;
        Ok(receipt)
    }

    pub fn clock(value: &Value) -> Result<()> {
        let text = Wire::text(value)?;
        let bytes = text.as_bytes();
        Wire::require(
            bytes.len() == 20
                && bytes.iter().enumerate().all(|(index, byte)| match index {
                    4 | 7 => *byte == b'-',
                    10 => *byte == b'T',
                    13 | 16 => *byte == b':',
                    19 => *byte == b'Z',
                    _ => byte.is_ascii_digit(),
                }),
            "invalid native start timestamp",
        )
    }

    fn material(value: &Value, fields: &[&str]) -> Result<()> {
        Wire::fields(value, fields)?;
        for field in fields {
            Wire::hex(&value[*field], 64)?;
        }
        Ok(())
    }

    fn validate(&self, subject: &Subject, policy: &Value) -> Result<()> {
        let p = &self.passed;
        Wire::fields(
            p,
            &[
                "schema",
                "policy",
                "implementation",
                "observer",
                "source",
                "publisher",
                "distribution",
                "declaration",
                "requirements",
                "prepared",
                "runner",
                "scene",
                "groups",
                "products",
                "images",
                "controls",
                "cleanup",
                "evidence",
                "intent",
            ],
        )?;
        Wire::require(
            p["schema"] == "wharf.windtun.pass/v1"
                && p["policy"] == policy["policy"]
                && p["implementation"] == policy["implementation"],
            "credential policy or implementation is invalidated",
        )?;
        Wire::matches(
            p,
            &json!({"source":subject.source, "declaration":subject.declaration, "observer":self.index["observer"], "intent":self.result["intent"], "evidence":self.result["evidence"], "cleanup":true}),
            "credential context, evidence or cleanup differs",
        )?;
        Wire::observer(&p["publisher"])?;
        Wire::hex(&p["distribution"], 64)?;
        Wire::hex(&p["prepared"], 64)?;
        let cases = &subject.declaration["specification"]["cases"];
        Wire::names(cases)?;
        Wire::fields(
            &p["requirements"],
            &["platform", "declared", "selected", "scope"],
        )?;
        Wire::require(
            p["requirements"]
                == json!({"platform":"linux-x86_64", "declared":cases, "selected":cases, "scope":"declared"}),
            "credential has incomplete or unsupported coverage",
        )?;
        Self::groups(&p["groups"], cases)?;
        Wire::fields(&p["runner"], &["version", "sha256", "pointer", "seal"])?;
        Wire::hex(&p["runner"]["sha256"], 64)?;
        Wire::hex(&p["runner"]["pointer"], 64)?;
        Wire::text(&p["runner"]["version"])?;
        Wire::fields(&p["controls"], &["probe"])?;
        let probe = Wire::text(&p["controls"]["probe"])?;
        let digest = probe
            .rsplit_once("@sha256:")
            .map(|(_, digest)| Value::String(digest.into()))
            .unwrap_or(Value::Null);
        Wire::hex(&digest, 64)?;
        Self::single(
            &p["products"],
            &["name", "channel", "version", "commit", "seal", "archive"],
        )?;
        Self::single(&p["images"], &["name", "reference", "digest"])?;
        Ok(())
    }

    pub fn single<'a>(value: &'a Value, fields: &[&str]) -> Result<&'a Value> {
        let list = value
            .as_array()
            .ok_or_else(|| super::Fault::refused("expected an identity list"))?;
        Wire::require(
            list.len() == 1,
            "only the recorded single-product combination is supported",
        )?;
        Wire::fields(&list[0], fields)?;
        Ok(&list[0])
    }

    fn groups(value: &Value, selected: &Value) -> Result<()> {
        let groups = value
            .as_array()
            .ok_or_else(|| super::Fault::refused("missing resolved groups"))?;
        Wire::require(!groups.is_empty(), "missing resolved groups")?;
        let mut digests = BTreeSet::new();
        let mut names = Vec::new();
        for group in groups {
            Wire::fields(group, &["digest", "cases"])?;
            Wire::require(
                digests.insert(Wire::hex(&group["digest"], 64)?),
                "duplicate group identity",
            )?;
            names.extend(Wire::names(&group["cases"])?);
        }
        Wire::names(&json!(names))?;
        names.sort();
        let mut wanted = Wire::names(selected)?;
        wanted.sort();
        Wire::require(
            names == wanted,
            "resolved groups do not cover the complete declaration",
        )
    }

    pub fn unchanged<R: Read>(&self, reader: &R) -> Result<()> {
        let result = Wire::document(reader, &format!("{}/result.json", self.attempt), true)?;
        let index = Wire::document(reader, &format!("{}/latest.json", self.root), true)?;
        Wire::require(
            result == self.result && index == self.index,
            "qualification discovery or result moved during readback",
        )
    }
}
