use super::{
    Read, Result,
    archive::Archive,
    receipt::Receipt,
    source::Subject,
    wire::{LIMIT, Wire},
};
use serde_json::{Value, json};

const RUNNER: &str = "https://releases.seatrial.perish.uk";

pub(super) struct World<'a, R> {
    reader: &'a R,
    subject: &'a Subject,
}

impl<'a, R: Read> World<'a, R> {
    pub fn new(reader: &'a R, subject: &'a Subject) -> Self {
        Self { reader, subject }
    }

    pub fn pins(&self) -> Result<Value> {
        let s = self.subject;
        let scenes = &s.declaration["specification"]["scenes"];
        let product = Wire::document(
            self.reader,
            &format!("{}/v1/channels/{}.json", s.authority, s.channel),
            false,
        )?;
        Wire::matches(
            &product,
            &json!({"product":s.product, "channel":s.channel, "releaseVersion":s.source["marker"], "commit":s.source["commit"]}),
            "selected product channel moved",
        )?;
        let image = Wire::text(&s.declaration["specification"]["image"])?;
        let reference = if !image.contains('@')
            && !image.rsplit('/').next().unwrap_or_default().contains(':')
        {
            format!(
                "{image}:{}",
                Wire::text(&s.source["marker"])?.trim_start_matches('v')
            )
        } else {
            image.to_string()
        };
        let digest = self.reader.image(&reference)?;
        let suffix = digest
            .strip_prefix("sha256:")
            .ok_or_else(|| super::Fault::refused("invalid runtime digest"))?;
        Wire::hex(&json!(suffix), 64)?;
        let repository = reference.split('@').next().unwrap_or_default();
        let repository = match repository.rsplit_once(':') {
            Some((root, _))
                if !repository
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .contains('/') =>
            {
                root
            }
            _ => repository,
        };
        let image = json!({"name":s.product, "reference":reference, "digest":format!("{repository}@{digest}")});
        Ok(json!({
            "product":product,
            "runner":Wire::document(self.reader, &format!("{RUNNER}/v1/channels/stable.json"), false)?,
            "scene":Wire::document(self.reader, &format!("{}/channels/{}/scenes/latest.json", Wire::text(&scenes["authority"])?, Wire::text(&scenes["channel"])?), false)?,
            "image":image,
        }))
    }

    pub fn verify(&self, passed: &Value) -> Result<Value> {
        let pins = self.pins()?;
        self.product(passed, &pins["product"])?;
        self.runner(passed, &pins["runner"])?;
        self.scene(&pins["scene"])?;
        Wire::require(
            passed["scene"] == pins["scene"] && passed["images"] == json!([pins["image"]]),
            "credential scene or image differs from current inputs",
        )?;
        Ok(pins)
    }

    fn product(&self, passed: &Value, pointer: &Value) -> Result<()> {
        let s = self.subject;
        let root = format!(
            "{}/v1/releases/{}/{}",
            s.authority,
            s.channel,
            Wire::text(&s.source["marker"])?
        );
        let record = Wire::document(self.reader, &format!("{root}/distribution.json"), false)?;
        Wire::fields(
            &record,
            &[
                "schema",
                "repository",
                "marker",
                "commit",
                "tree",
                "media",
                "state",
                "attempt",
                "completed",
                "resolution",
                "trial",
            ],
        )?;
        Wire::matches(
            &record,
            &json!({"schema":1, "state":"complete", "trial":s.declaration, "completed":passed["publisher"]}),
            "qualification distribution or declaration differs",
        )?;
        Wire::require(
            Wire::digest(&record)? == passed["distribution"],
            "qualification distribution digest differs",
        )?;
        for field in ["repository", "marker", "commit", "tree"] {
            Wire::require(
                record[field] == s.source[field],
                "distribution source differs",
            )?;
        }
        Wire::observer(&record["attempt"])?;
        Wire::observer(&record["completed"])?;
        Wire::fields(
            &record["media"],
            &[
                "binaries", "cargo", "oci", "npm", "chart", "cfworker", "channel",
            ],
        )?;
        for state in record["media"]
            .as_object()
            .into_iter()
            .flat_map(|v| v.values())
        {
            Wire::text(state)?;
        }
        Wire::require(
            record["resolution"].is_object()
                && ["published", "present"]
                    .iter()
                    .any(|state| record["media"]["binaries"] == *state),
            "publication has no installed artifact",
        )?;
        let (seal, raw) = Archive::seal(self.reader, pointer, &s.authority)?;
        let artifact = &seal["artifacts"]["linux-x64"];
        let product = Receipt::single(
            &passed["products"],
            &["name", "channel", "version", "commit", "seal", "archive"],
        )?;
        Wire::require(
            *product
                == json!({"name":s.product, "channel":s.channel, "version":s.source["marker"], "commit":s.source["commit"], "seal":Wire::sha(&raw), "archive":artifact["sha256"]}),
            "tested artifact identity differs",
        )?;
        let files = Archive::files(
            &Wire::read(self.reader, artifact, &s.authority)?,
            &s.installed,
        )?;
        for binary in files.values() {
            Archive::identity(binary, &s.source)?;
        }
        Ok(())
    }

    fn runner(&self, passed: &Value, pointer: &Value) -> Result<()> {
        Wire::require(
            pointer["product"] == "seatrial" && pointer["channel"] == "stable",
            "runner is not official stable",
        )?;
        let (seal, _) = Archive::seal(self.reader, pointer, RUNNER)?;
        Wire::require(
            passed["runner"]["pointer"] == Wire::digest(pointer)?
                && passed["runner"]["seal"] == pointer["seal"]
                && passed["runner"]["version"]
                    == format!("seatrial {}", Wire::text(&pointer["releaseVersion"])?),
            "credential runner moved from official stable",
        )?;
        let commit = Wire::text(&seal["commit"])?;
        let identity = Wire::document(
            self.reader,
            &format!("https://api.github.com/repos/PerishLab/seatrial/git/commits/{commit}"),
            false,
        )?;
        Wire::require(identity["sha"] == commit, "runner source differs")?;
        Wire::hex(&identity["tree"]["sha"], 40)?;
        let source = json!({"repository":"PerishLab/seatrial", "marker":seal["releaseVersion"], "commit":commit, "tree":identity["tree"]["sha"]});
        let files = Archive::files(
            &Wire::read(self.reader, &seal["artifacts"]["linux-x64"], RUNNER)?,
            &["seatrial".into()],
        )?;
        let binary = &files["seatrial"];
        Wire::require(
            Wire::sha(binary) == passed["runner"]["sha256"],
            "actual runner bytes differ from tested runner",
        )?;
        Archive::identity(binary, &source)
    }

    fn scene(&self, value: &Value) -> Result<()> {
        use crate::depot::v3::{Kind, Manifest, Pointer, Route};
        let scenes = &self.subject.declaration["specification"]["scenes"];
        let authority = Wire::text(&scenes["authority"])?;
        let pointer = Pointer::parse(&Wire::encode(value)?).map_err(super::Fault::refused)?;
        Wire::require(
            pointer.kind == Kind::Scene
                && pointer.product == scenes["product"]
                && pointer.channel == scenes["channel"],
            "current scene projection differs",
        )?;
        let url = crate::depot::v3::manifest(
            authority,
            Route::new(&pointer.channel, Kind::Scene, &pointer.version),
            &pointer.generation,
        )
        .map_err(super::Fault::refused)?;
        Wire::require(
            value["manifest"]["url"] == url,
            "scene manifest has a foreign generation",
        )?;
        let raw = Wire::read(self.reader, &value["manifest"], authority)?;
        Wire::parse(&raw, false)?;
        let manifest = Manifest::parse(&raw).map_err(super::Fault::refused)?;
        pointer
            .bind(&manifest, &raw)
            .map_err(super::Fault::refused)?;
        let size = manifest
            .objects
            .iter()
            .try_fold(0_u64, |total, object| total.checked_add(object.size));
        Wire::require(
            manifest.objects.len() <= 1024
                && size.is_some_and(|value| value <= (16 * LIMIT) as u64),
            "scene material exceeds bounds",
        )?;
        let base = url
            .strip_suffix("manifest.json")
            .ok_or_else(|| super::Fault::refused("scene manifest location differs"))?;
        for object in &manifest.objects {
            Wire::require(object.size <= LIMIT as u64, "scene object exceeds bounds")?;
            let raw = self.reader.bytes(
                &format!("{base}objects/{}", object.path),
                object.size as usize,
            )?;
            manifest
                .verify(&object.path, &raw, object.executable)
                .map_err(super::Fault::refused)?;
        }
        Ok(())
    }
}
