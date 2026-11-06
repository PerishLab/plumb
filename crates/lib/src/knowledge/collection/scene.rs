use super::{bundle::Index, wire::Wire};
use crate::seat::depot::v3::{self, Manifest, Pointer, Route};
use crate::semantics::{Kind, Material, subjects::Subjects};
use crate::windtun::{
    Scenes,
    qualification::{Fault, Read},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Reader<'a, R>(pub &'a R);

#[derive(Debug, Serialize)]
pub struct Scene {
    pub pointer: Pointer,
    pub manifest: Manifest,
    pub materials: Vec<Material>,
    pub cases: Vec<String>,
    pub subjects: Option<Subjects>,
}

impl<R: Read> Reader<'_, R> {
    fn pointer(&self, scenes: &Scenes, url: &str) -> Result<Pointer, Fault> {
        let raw = self.0.bytes(url, Wire::LIMIT)?;
        Wire::require(
            !raw.is_empty() && raw.len() <= Wire::LIMIT,
            "Scene pointer exceeds bounds",
        )?;
        let held = Pointer::parse(&raw).map_err(Fault::refused)?;
        Wire::require(
            held.product == scenes.product && held.channel == scenes.channel,
            "Scene pointer identity differs",
        )?;
        Wire::require(
            held.kind == v3::Kind::Scene,
            "Scene pointer names another material kind",
        )?;
        let expected = v3::manifest(
            &scenes.authority,
            Route::new(&held.channel, held.kind, &held.version),
            &held.generation,
        )
        .map_err(Fault::refused)?;
        Wire::require(
            held.manifest.url == expected,
            "Scene manifest has a foreign source",
        )?;
        Ok(held)
    }

    fn discovery(scenes: &Scenes) -> Result<String, Fault> {
        Wire::name(&scenes.product)?;
        Wire::require(
            ["stable", "alpha", "beta", "rc"].contains(&scenes.channel.as_str()),
            "Scene requires a channel",
        )?;
        Wire::require(
            scenes.authority == format!("https://depot.{}.perish.uk", scenes.product),
            "Scene requires its canonical first-party authority",
        )?;
        Ok(format!(
            "{}/channels/{}/scenes/latest.json",
            scenes.authority, scenes.channel
        ))
    }

    fn standing(scenes: &Scenes, pointer: &Pointer) -> Result<String, Fault> {
        let route = Route::new(&pointer.channel, pointer.kind, &pointer.version);
        Ok(format!(
            "{}/{}",
            scenes.authority,
            v3::latest(route).map_err(Fault::refused)?
        ))
    }

    pub fn inspect(&self, scenes: &Scenes) -> Result<Scene, Fault> {
        let discovery = Self::discovery(scenes)?;
        let pointer = self.pointer(scenes, &discovery)?;
        Wire::require(
            self.pointer(scenes, &Self::standing(scenes, &pointer)?)? == pointer,
            "Scene channel and version standing differ",
        )?;
        Wire::require(
            pointer.manifest.size <= Wire::LIMIT as u64,
            "Scene manifest exceeds bounds",
        )?;
        let raw = self.0.bytes(&pointer.manifest.url, Wire::LIMIT)?;
        Wire::require(
            raw.len() as u64 == pointer.manifest.size,
            "Scene manifest size differs",
        )?;
        let manifest = Manifest::parse(&raw).map_err(Fault::refused)?;
        pointer.bind(&manifest, &raw).map_err(Fault::refused)?;
        Wire::require(
            manifest.objects.len() <= 2048,
            "Scene object inventory exceeds bounds",
        )?;
        let objects = self.objects(&pointer, &manifest)?;
        let paths: Vec<_> = objects.keys().cloned().collect();
        let bundle = objects
            .get("bundle.toml")
            .ok_or_else(|| Fault::refused("formal Scene has no bundle index"))?;
        let index = Index::parse(bundle, &paths)?;
        let cases = index
            .cases
            .iter()
            .map(|case| case.id.clone())
            .collect::<Vec<_>>();
        let subjects = objects
            .get(Subjects::PATH)
            .map(|raw| Subjects::parse(raw).map_err(Fault::refused))
            .transpose()?;
        if let Some(mapping) = &subjects {
            Wire::require(
                mapping.product == scenes.product,
                "subject metadata names another Scene producer",
            )?;
            mapping.bind(&cases, &paths).map_err(Fault::refused)?;
        }
        let mut materials = Vec::new();
        for object in &manifest.objects {
            let bytes = &objects[&object.path];
            materials.push(
                Material::new(
                    Kind::Helper,
                    format!("scene/{}", object.path),
                    bytes,
                    object.executable,
                )
                .map_err(Fault::refused)?,
            );
        }
        let raw = serde_json::to_vec(&pointer)
            .map_err(|_| Fault::refused("cannot encode Scene identity"))?;
        materials.push(
            Material::new(Kind::Runtime, "scene.json".into(), &raw, false)
                .map_err(Fault::refused)?,
        );
        Ok(Scene {
            pointer,
            manifest,
            materials,
            cases,
            subjects,
        })
    }

    fn objects(
        &self,
        pointer: &Pointer,
        manifest: &Manifest,
    ) -> Result<BTreeMap<String, Vec<u8>>, Fault> {
        let base = pointer
            .manifest
            .url
            .strip_suffix("manifest.json")
            .ok_or_else(|| Fault::refused("Scene manifest has no anchored base"))?;
        let mut objects = BTreeMap::new();
        let mut aliases = BTreeSet::new();
        let mut total = 0usize;
        for object in &manifest.objects {
            Wire::path(&object.path)?;
            Wire::require(
                aliases.insert(object.path.to_ascii_lowercase()),
                "Scene paths have portable aliases",
            )?;
            let size = usize::try_from(object.size)
                .map_err(|_| Fault::refused("Scene object size exceeds platform bounds"))?;
            total = total
                .checked_add(size)
                .ok_or_else(|| Fault::refused("Scene size overflow"))?;
            Wire::require(total <= Wire::TOTAL, "Scene total material budget exceeded")?;
            let bytes = self
                .0
                .bytes(&format!("{base}objects/{}", object.path), size)?;
            manifest
                .verify(&object.path, &bytes, object.executable)
                .map_err(Fault::refused)?;
            objects.insert(object.path.clone(), bytes);
        }
        Ok(objects)
    }

    pub fn unchanged(&self, scenes: &Scenes, scene: &Scene) -> Result<(), Fault> {
        let current = self.pointer(scenes, &Self::discovery(scenes)?)?;
        let standing = self.pointer(scenes, &Self::standing(scenes, &scene.pointer)?)?;
        Wire::require(
            current == scene.pointer && standing == scene.pointer,
            "Scene discovery moved during observation",
        )
    }
}
