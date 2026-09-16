use crate::command::release::ReleaseMarker;
use plumb::rule::Completion;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Contract {
    marker: String,
    digest: String,
    resources: BTreeSet<String>,
}

impl Contract {
    pub(super) fn new(marker: &ReleaseMarker) -> Result<Self, String> {
        let surface: Value =
            serde_json::from_str(&crate::command::release::plan::surface(marker.spec())?)
                .map_err(|error| error.to_string())?;
        let mut resources = BTreeSet::new();
        for row in surface["publication"]["include"]
            .as_array()
            .ok_or("Ship surface has no publication set")?
        {
            resources.insert(
                row["action"]
                    .as_str()
                    .ok_or("Ship surface has an unnamed publication")?
                    .to_string(),
            );
        }
        if marker.spec().binary() {
            resources.insert("ship/binary".into());
        }
        let identity = marker.digest()?;
        let contract = json!({
            "schema": "plumb.ship-contract/v1",
            "marker": identity,
            "resources": resources,
        });
        Ok(Self {
            marker: identity,
            digest: crate::command::release::record::sha(
                &serde_json::to_vec(&contract).map_err(|error| error.to_string())?,
            ),
            resources,
        })
    }

    fn read(&self, body: &[u8]) -> Result<Completion, String> {
        let completion: Completion = serde_json::from_slice(body)
            .map_err(|error| format!("invalid Ship completion: {error}"))?;
        completion.verify(&self.marker, &self.digest, &self.resources)?;
        Ok(completion)
    }
}

pub(in crate::command) fn completed(marker: &ReleaseMarker) -> Result<bool, String> {
    if !marker.independent() {
        return Err("Ship completion requires an independent release marker".into());
    }
    let url = format!("{}/{}", marker.authority, key(marker));
    let Some(body) = crate::command::release::verify::Surface(&url).bytes()? else {
        return Ok(false);
    };
    Contract::new(marker)?.read(&body)?;
    Ok(true)
}

pub(super) fn key(marker: &ReleaseMarker) -> String {
    format!("v1/releases/{}/{}/ship.json", marker.channel, marker.marker)
}

pub(super) fn publish(
    marker: &ReleaseMarker,
    authority: &plumb::rig::Authority,
    evidence: &BTreeMap<String, Value>,
) -> Result<Vec<u8>, String> {
    let contract = Contract::new(marker)?;
    if evidence.keys().cloned().collect::<BTreeSet<_>>() != contract.resources {
        return Err("Ship evidence does not cover its exact resource set".into());
    }
    let authority = super::support::authority(authority, &marker.product)?;
    let control = plumb::bucket::Control::new(
        authority.access,
        authority.secret,
        authority.bucket,
        authority.endpoint,
    )?;
    let mut resources = BTreeMap::new();
    for (action, proof) in evidence {
        if proof["schema"] != "plumb.ship-resource/v1"
            || proof["marker"] != contract.marker
            || proof["action"] != *action
        {
            return Err("publication proof differs from its marker and action".into());
        }
        if proof["source"].as_str().is_none_or(str::is_empty) {
            return Err("publication proof has no resource identity".into());
        }
        let content = proof["content"]
            .as_str()
            .ok_or("publication proof has no content digest")?;
        if content.len() != 64
            || !content
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("publication proof has an invalid content digest".into());
        }
        let body = serde_json::to_vec(proof).map_err(|error| error.to_string())?;
        let digest = crate::command::release::record::sha(&body);
        let path = format!(
            "v1/releases/{}/{}/proofs/{digest}.json",
            marker.channel, marker.marker
        );
        write(&control, &path, &body)?;
        resources.insert(
            action.clone(),
            plumb::rule::Resource {
                source: format!("{}/{path}", marker.authority),
                digest,
            },
        );
    }
    let completion = Completion {
        schema: "plumb.ship-completion/v1".into(),
        marker: contract.marker.clone(),
        contract: contract.digest.clone(),
        resources,
    };
    completion.verify(&contract.marker, &contract.digest, &contract.resources)?;
    let body = serde_json::to_vec(&completion).map_err(|error| error.to_string())?;
    write(&control, &key(marker), &body)?;
    let public =
        crate::command::release::verify::Surface(&format!("{}/{}", marker.authority, key(marker)))
            .bytes()?
            .ok_or("Ship completion is not publicly readable")?;
    if public != body {
        return Err("published Ship completion drift".into());
    }
    Ok(body)
}

fn write(control: &plumb::bucket::Control, key: &str, body: &[u8]) -> Result<(), String> {
    use plumb::bucket::{Condition, Outcome, Policy};
    let written = control.write(
        key,
        body,
        Policy {
            media: "application/json",
            cache: "public, max-age=31536000, immutable",
        },
        Condition::Absent,
    );
    match control.read(key)? {
        Outcome::Held(object) if object.body == body => Ok(()),
        Outcome::Held(_) => Err("immutable Ship evidence conflict".into()),
        _ => Err(written
            .err()
            .unwrap_or_else(|| "Ship evidence write is not readable".into())),
    }
}
