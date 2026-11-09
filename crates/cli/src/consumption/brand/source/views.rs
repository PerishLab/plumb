use super::super::svelte::Mark;
use super::{App, Source};
use std::path::{Path, PathBuf};

pub(super) fn verify(
    root: &Path,
    app: &App,
    sources: &[(PathBuf, Vec<Mark>)],
) -> Result<(), String> {
    if app.setup.base != "/"
        || app.setup.options.get("login") != Some(&serde_json::Value::Bool(false))
    {
        return Err("Views carrier requires the root Vite base and explicit login:false".into());
    }
    let directory = root.join(&app.path);
    let view = directory.join("src/views/index.svelte");
    if Source(&directory.join("src/views")).files()? != [view.clone()] {
        return Err("Views carrier currently qualifies one root view only".into());
    }
    let mut chosen = Vec::new();
    for (file, marks) in sources {
        if !marks.is_empty() && *file != view {
            return Err(
                "brand component outside the qualified root view is unreachable in this carrier"
                    .into(),
            );
        }
        chosen.extend(marks);
    }
    let [mark] = chosen.as_slice() else {
        return Err("Views root needs exactly one imported brand component".into());
    };
    if !mark.root {
        return Err(
            "Views brand must be unconditional under plain formal Shell/Frame layouts".into(),
        );
    }
    if let Some(head) = app.setup.options.get("brand")
        && head.get("product").and_then(serde_json::Value::as_str) != Some(mark.product.as_str())
    {
        return Err("Views root brand differs from its declared head product".into());
    }
    let document = super::document::read(&text(&directory, "index.html")?)?;
    let script = document
        .script
        .strip_prefix('/')
        .ok_or("Views source script is not local")?;
    if !script.starts_with("src/")
        || !["ts", "js"].contains(
            &Path::new(script)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default(),
        )
    {
        return Err("Views entry must be a tracked local JS/TS source module".into());
    }
    super::entry::verify(&text(&directory, script)?)
}

pub(super) fn built(root: &Path, app: &App) -> Result<super::Entry, String> {
    let directory = root.join(&app.path);
    let source = super::document::read(&text(&directory, "index.html")?)?;
    let output = directory.join(&app.setup.output);
    let actual = super::document::read(&text(&output, "index.html")?)?;
    let path = actual
        .script
        .strip_prefix('/')
        .ok_or("Views bundle is not local")?;
    if !["js", "mjs"].contains(
        &Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
    ) {
        return Err("Views output has no local JavaScript module".into());
    }
    let bytes = super::super::material::regular(&output, path)?;
    if bytes.is_empty() {
        return Err("Views JavaScript output is empty".into());
    }
    Ok(super::Entry {
        source: source.script,
        output: actual.script,
        digest: super::super::material::digest(&bytes),
        view: "src/views/index.svelte".into(),
        target: "root".into(),
    })
}

fn text(root: &Path, path: &str) -> Result<String, String> {
    String::from_utf8(super::super::material::regular(root, path)?)
        .map_err(|error| error.to_string())
}
