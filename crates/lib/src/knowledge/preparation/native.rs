use super::super::{Authority, Inspector, Native, Reference, World, policy, producer, wire::Wire};
use super::Payload;
use crate::windtun::qualification::{Fault, Read};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Preparation {
    pub schema: String,
    pub native: Native,
    pub world: Reference,
    pub material: Reference,
}

impl Preparation {
    pub const SCHEMA: &str = "wharf.release.semantics.world/v1";

    pub fn parse(raw: &[u8]) -> Result<Self, Fault> {
        let held: Self = Wire::parse(raw)?;
        Wire::require(
            held.schema == Self::SCHEMA,
            "unsupported native world preparation",
        )?;
        held.native.validate()?;
        Ok(held)
    }
}

#[derive(Debug, Serialize)]
pub struct Prepared {
    pub preparation: Preparation,
    pub authority: Authority,
    pub producer: Value,
    pub job: Value,
    pub world: World,
    #[serde(serialize_with = "Prepared::material")]
    pub material: Payload,
    pub authentication: bool,
    pub publication: bool,
}

impl Prepared {
    fn material<S: serde::Serializer>(payload: &Payload, serializer: S) -> Result<S::Ok, S::Error> {
        json!({"source":payload.source,"head":payload.head,"tree":payload.tree,
            "sha256":payload.sha256,"files":payload.files.len()})
        .serialize(serializer)
    }
}

impl<R: Read> Inspector<R> {
    pub fn prepared(&self, preparation: &Preparation, product: &str) -> Result<Prepared, Fault> {
        Wire::require(
            preparation.schema == Preparation::SCHEMA,
            "unsupported native world preparation",
        )?;
        preparation.native.validate()?;
        preparation.world.validate(product)?;
        preparation.material.validate(product)?;
        let authority = policy::Reader(&self.0).current()?;
        let producer = producer::Reader(&self.0);
        producer.lineage(&preparation.native.commit, &authority.source.commit)?;
        let run = producer.run(&preparation.native)?;
        let step = format!(
            "[prepare] world {} {}",
            preparation.world.sha256, preparation.material.sha256
        );
        let job = producer.role(&preparation.native, "[prepare] semantic world", &step)?;
        let raw = preparation.world.bytes(&self.0, product)?;
        let world = World::parse(&raw)?;
        Wire::require(
            world.source.repository == format!("PerishLab/{product}"),
            "prepared world names another product",
        )?;
        let bytes = preparation.material.bytes(&self.0, product)?;
        let material = Payload::parse(&bytes, &world)?;
        producer.run(&preparation.native)?;
        Wire::require(
            producer.role(&preparation.native, "[prepare] semantic world", &step)? == job,
            "native world preparation job moved during verification",
        )?;
        Wire::require(
            preparation.world.bytes(&self.0, product)? == raw
                && preparation.material.bytes(&self.0, product)? == bytes,
            "prepared world or effective source moved during verification",
        )?;
        Wire::require(
            policy::Reader(&self.0).current()? == authority,
            "native world preparation authority moved during verification",
        )?;
        Ok(Prepared {
            preparation: preparation.clone(),
            authority,
            producer: run,
            job,
            world,
            material,
            authentication: true,
            publication: false,
        })
    }
}
