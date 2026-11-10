use super::super::{Fault, Read, Result, archive::Archive, source::Source, wire::Wire};
use crate::windtun::Service;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) struct Services<'a, R> {
    pub reader: &'a R,
    pub declaration: &'a Value,
    pub target: &'a str,
}

impl<R: Read> Services<'_, R> {
    pub fn pins(&self) -> Result<Value> {
        let mut pins = serde_json::Map::new();
        let Some(services) = self.declaration.get("services") else {
            return Ok(Value::Object(pins));
        };
        let services = services
            .as_object()
            .ok_or_else(|| Fault::refused("invalid services"))?;
        Wire::require(
            !services.is_empty() && services.len() <= 4,
            "invalid service count",
        )?;
        for (name, value) in services {
            Wire::require(name != self.target, "service repeats the published target")?;
            let service: Service = serde_json::from_value(value.clone())
                .map_err(|_| Fault::refused("invalid service declaration"))?;
            service.validate(name).map_err(Fault::refused)?;
            pins.insert(name.clone(), self.service(name, &service)?);
        }
        Ok(Value::Object(pins))
    }

    fn service(&self, name: &str, service: &Service) -> Result<Value> {
        let authority = &service.authority;
        let pointer = Wire::document(
            self.reader,
            &format!("{authority}/v1/channels/stable.json"),
            false,
        )?;
        Wire::matches(
            &pointer,
            &json!({"product":name,"channel":"stable"}),
            "service is not canonical stable",
        )?;
        let marker = Wire::text(&pointer["releaseVersion"])?;
        let version = semver::Version::parse(marker.strip_prefix('v').unwrap_or_default())
            .map_err(|_| Fault::refused("invalid service marker"))?;
        Wire::require(
            version.pre.is_empty() && version.build.is_empty() && marker == format!("v{version}"),
            "service marker is not formal stable",
        )?;
        let source = Source(self.reader);
        let repository = format!("PerishLab/{name}");
        let identity = source.repository(&repository)?;
        let native = source.commit(&repository, marker)?;
        let (seal, _) = Archive::seal(self.reader, &pointer, authority)?;
        let distribution = Wire::document(
            self.reader,
            &format!("{authority}/v1/releases/stable/{marker}/distribution.json"),
            false,
        )?;
        super::plan::admit(&distribution)?;
        Wire::observer(&distribution["attempt"])?;
        Wire::observer(&distribution["completed"])?;
        Wire::fields(
            &distribution["media"],
            &[
                "binaries", "cargo", "oci", "npm", "chart", "cfworker", "channel",
            ],
        )?;
        Wire::require(
            distribution["resolution"].is_object(),
            "service resolution is absent",
        )?;
        Wire::matches(
            &distribution,
            &json!({"schema":1,"state":"complete","repository":repository,"marker":marker,"commit":pointer["commit"],"tree":native["source"]["tree"]}),
            "service distribution is not complete at its native source",
        )?;
        Wire::require(
            native["source"]["commit"] == pointer["commit"],
            "service tag differs from pointer",
        )?;
        Wire::require(
            distribution["media"]["binaries"] == "published"
                || distribution["media"]["binaries"] == "present",
            "service has no distributed binaries",
        )?;
        let archive = &seal["artifacts"][&service.placement.artifact];
        Wire::read(self.reader, archive, authority)?;
        Ok(
            json!({"declaration":service,"pointer":pointer,"seal":seal,"distribution":distribution,"source":native["source"],"tag":native["tag"],"repository":identity}),
        )
    }

    pub fn verify(&self, passed: &Value, pins: &Value) -> Result<()> {
        Wire::require(
            passed["services"] == *pins,
            "service world moved from qualified combination",
        )?;
        for (name, service) in pins.as_object().into_iter().flat_map(|value| value.iter()) {
            let product = Self::product(passed, name)?;
            let pointer = &service["pointer"];
            let placement = &service["declaration"]["placement"];
            let artifact = Wire::text(&placement["artifact"])?;
            Wire::require(
                *product
                    == json!({"name":name,"channel":"stable","version":pointer["releaseVersion"],"commit":pointer["commit"],"seal":pointer["seal"]["sha256"],"archive":service["seal"]["artifacts"][artifact]["sha256"],"placement":placement}),
                "consumed service package or placement differs",
            )?;
        }
        Ok(())
    }
}

impl<R> Services<'_, R> {
    pub fn product<'a>(passed: &'a Value, name: &str) -> Result<&'a Value> {
        let products = passed["products"]
            .as_array()
            .ok_or_else(|| Fault::refused("missing products"))?;
        let found: Vec<_> = products
            .iter()
            .filter(|value| value["name"] == name)
            .collect();
        Wire::require(found.len() == 1, "product is absent or duplicated")?;
        Ok(found[0])
    }

    pub fn records(passed: &Value, declaration: &Value, target: &str) -> Result<()> {
        let services = declaration["services"]
            .as_object()
            .ok_or_else(|| Fault::refused("missing service declaration"))?;
        Wire::require(
            !services.is_empty() && services.len() <= 4 && !services.contains_key(target),
            "invalid service combination",
        )?;
        let products = passed["products"]
            .as_array()
            .ok_or_else(|| Fault::refused("missing products"))?;
        Wire::require(
            products.len() == services.len() + 1,
            "product combination coverage differs",
        )?;
        let mut names = BTreeSet::new();
        for product in products {
            let name = Wire::text(&product["name"])?;
            Wire::require(names.insert(name), "duplicate product identity")?;
            let mut fields = vec!["name", "channel", "version", "commit", "seal", "archive"];
            if name != target {
                let service = services
                    .get(name)
                    .ok_or_else(|| Fault::refused("undeclared service"))?;
                fields.push("placement");
                Wire::require(
                    product["placement"] == service["placement"],
                    "service placement differs from declaration",
                )?;
            }
            Wire::fields(product, &fields)?;
        }
        Wire::require(names.contains(target), "published target is absent")
    }
}
