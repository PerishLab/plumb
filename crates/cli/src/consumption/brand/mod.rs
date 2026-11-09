mod head;
mod material;
mod receipt;
mod source;
mod svelte;

use material::Materials;
use receipt::Catalog;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    #[serde(default)]
    mode: Mode,
    #[serde(default)]
    copy: Vec<Copy>,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Mode {
    #[default]
    Html,
    Views,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Copy {
    asset: String,
    output: String,
}

#[derive(Serialize)]
pub(crate) struct Prepared {
    declaration: Declaration,
    apps: Vec<source::App>,
    materials: Materials,
    inputs: String,
}

#[derive(Serialize)]
pub(crate) struct Report {
    schema: &'static str,
    apps: Vec<App>,
    copies: usize,
    origins: Vec<plumb::packages::Origin>,
    inputs: String,
    publications: Vec<material::Publication>,
    limits: &'static str,
}

#[derive(Serialize)]
struct App {
    path: PathBuf,
    images: usize,
    marks: usize,
    mode: Mode,
    rendering: &'static str,
    browser: &'static str,
    entry: Option<source::Entry>,
}

impl Prepared {
    pub(crate) fn capture(
        root: &Path,
        resolution: &plumb::packages::Resolution,
    ) -> Result<Option<Self>, String> {
        let Some(declaration) = declared(root)? else {
            return Ok(None);
        };
        let apps = source::Source(root).apps(declaration.mode)?;
        if apps.is_empty() && declaration.copy.is_empty() {
            return Err("brand declaration has no qualified app or materialized copy".into());
        }
        let materials = Materials::capture(root, resolution, !apps.is_empty())?;
        let inputs = source::Source(root).inputs(&apps)?;
        let held = Self {
            declaration,
            apps,
            materials,
            inputs,
        };
        let catalog = Catalog::read(&held.materials)?;
        held.sources(root, &catalog)?;
        held.copies(root, &catalog)?;
        Ok(Some(held))
    }

    pub(crate) fn bind(&self, input: &str) -> Result<String, String> {
        let bytes = serde_json::to_vec(&(input, self)).map_err(|error| error.to_string())?;
        Ok(plumb::depot::sha(&bytes))
    }

    pub(crate) fn verify(&self, root: &Path) -> Result<Report, String> {
        let catalog = Catalog::read(&self.materials)?;
        let marks = self.sources(root, &catalog)?;
        let mut apps = Vec::new();
        for (app, count) in self.apps.iter().zip(marks) {
            self.materials.installed(&root.join(&app.path))?;
            let images = catalog.verify(root, app, &self.materials)?;
            apps.push(App {
                path: app.path.clone(),
                images,
                marks: count,
                mode: app.mode,
                rendering: match app.mode {
                    Mode::Html => "html-images-observed",
                    Mode::Views => "independent-browser-required",
                },
                browser: "unverified",
                entry: match app.mode {
                    Mode::Html => None,
                    Mode::Views => Some(source::Source(root).built(app)?),
                },
            });
        }
        self.copies(root, &catalog)?;
        Ok(Report {
            schema: "plumb.brand/v2",
            apps,
            copies: self.declaration.copy.len(),
            origins: self.materials.origins.clone(),
            inputs: self.inputs.clone(),
            publications: self.materials.publications.clone(),
            limits: "Bounded formal origin, source selection and unchanged official image output checks. HTML mode observes generated image tags; views mode proves only the closed root-entry relation and local module presence/digest, never compiled-source equivalence or runtime rendering. Browser display, runtime requests, arbitrary image semantics, other entry programs/hosts remain unverified.",
        })
    }

    fn copies(&self, root: &Path, catalog: &Catalog) -> Result<(), String> {
        if self.declaration.copy.is_empty() {
            return Ok(());
        }
        let snapshot = plumb::snapshot::Snapshot::read(root).map_err(|error| error.to_string())?;
        for copy in &self.declaration.copy {
            let entry = snapshot
                .entries()
                .iter()
                .find(|entry| entry.path() == copy.output)
                .ok_or("materialized brand copy is outside tracked source")?;
            if !["100644", "100755"].contains(&entry.mode()) {
                return Err("materialized brand copy is not tracked regular source".into());
            }
            let asset = catalog
                .assets
                .get(&copy.asset)
                .ok_or("materialized copy selects an unknown formal asset")?;
            let actual = material::regular(root, &copy.output)?;
            if catalog.files.get(&asset.file) != Some(&actual) {
                return Err(format!(
                    "materialized brand copy {} differs from the formal image",
                    copy.output
                ));
            }
        }
        Ok(())
    }

    fn sources(&self, root: &Path, catalog: &Catalog) -> Result<Vec<usize>, String> {
        let mut counts = Vec::new();
        for app in &self.apps {
            let mut count = 0;
            let mut sources = Vec::new();
            for file in source::Source(&root.join(&app.path).join("src")).files()? {
                let text = std::fs::read_to_string(&file).map_err(|error| error.to_string())?;
                let marks =
                    svelte::read(&text).map_err(|error| format!("{}: {error}", file.display()))?;
                for mark in &marks {
                    mark.verify(catalog, &file)?;
                }
                count += marks.len();
                sources.push((file, marks));
            }
            if app.mode == Mode::Views {
                source::Source(root).views(app, &sources)?;
            }
            if count == 0 {
                return Err(format!(
                    "{} has no qualified imported brand component; a dependency/head claim alone is not component adoption",
                    app.path.display()
                ));
            }
            counts.push(count);
        }
        Ok(counts)
    }
}

pub(crate) fn prepare(
    action: (&str, String),
    root: &Path,
    resolution: Option<&plumb::packages::Resolution>,
) -> Result<(String, Option<Prepared>), String> {
    let (name, input) = action;
    if name != "guard/web" {
        return Ok((input, None));
    }
    let prepared = Prepared::capture(
        root,
        resolution.ok_or("brand Guard needs a captured package world")?,
    )?;
    let input = match &prepared {
        Some(brand) => brand.bind(&input)?,
        None => input,
    };
    Ok((input, prepared))
}

fn declared(root: &Path) -> Result<Option<Declaration>, String> {
    let text =
        std::fs::read_to_string(root.join("plumb.toml")).map_err(|error| error.to_string())?;
    let document: toml::Value = toml::from_str(&text).map_err(|error| error.to_string())?;
    document
        .get("brand")
        .map(|value| {
            value
                .clone()
                .try_into()
                .map_err(|error| format!("brand declaration is unqualified: {error}"))
        })
        .transpose()
}

pub(crate) fn admit(manifest: Option<&str>, web: bool) -> Result<(), String> {
    let document: toml::Value =
        toml::from_str(manifest.unwrap_or_default()).map_err(|error| error.to_string())?;
    if document.get("brand").is_some() && !web {
        return Err("declared brand Guard currently requires a pnpm web action; other package/host worlds remain unqualified".into());
    }
    Ok(())
}

pub(crate) fn run(root: PathBuf, json: bool) -> i32 {
    let result = (|| {
        let resolution = plumb::packages::locked(&root, "brand")?;
        let prepared = Prepared::capture(&root,&resolution)?.ok_or("brand contract is not activated: declare [brand] in plumb.toml; absent declaration does not qualify adoption")?;
        prepared.verify(&root)
    })();
    match result {
        Ok(report) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                println!(
                    "brand: {} app(s), {} unchanged materialized copies; bounded origin/output verification passed",
                    report.apps.len(),
                    report.copies
                );
            }
            0
        }
        Err(error) => refused(error, json),
    }
}

fn refused(error: String, json: bool) -> i32 {
    if json {
        let message =
            serde_json::json!({"schema":"plumb.brand/v2","status":"unqualified","reason":error});
        println!("{message}");
    } else {
        eprintln!("plumb brand: {error}");
    }
    1
}

pub(crate) fn verify(prepared: Option<&Prepared>, root: &Path) -> Result<(), String> {
    if let Some(prepared) = prepared {
        let report = prepared.verify(root)?;
        crate::diagnostic::event("brand.verified", || serde_json::json!(report));
    }
    Ok(())
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod shared;
