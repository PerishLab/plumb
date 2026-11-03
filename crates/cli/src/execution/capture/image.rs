use std::io::Write;

pub(crate) struct Image {
    _root: tempfile::TempDir,
    tool: plumb::config::Tool,
}

impl Image {
    pub fn new() -> Result<Self, String> {
        if !cfg!(target_os = "linux") {
            return Err("Cargo launch-context capture is qualified on Linux only".into());
        }
        let root = tempfile::tempdir().map_err(|error| error.to_string())?;
        super::session::protect(root.path())?;
        let path = root.path().join("runner");
        let mut source =
            std::fs::File::open("/proc/self/exe").map_err(|error| error.to_string())?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| error.to_string())?;
        std::io::copy(&mut source, &mut output).map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
        drop(output);
        executable(&path)?;
        let tool = plumb::config::Tool::at(super::context::directory(&path)?)?;
        Ok(Self { _root: root, tool })
    }

    pub fn carrier(&self) -> Result<Vec<String>, String> {
        if plumb::config::Tool::at(self.tool.path.clone())? != self.tool {
            return Err("owned Guard runner image changed".into());
        }
        Ok(vec![
            self.tool
                .path
                .to_str()
                .ok_or("Guard runner image is not Unicode")?
                .into(),
            "runner".into(),
        ])
    }
}

#[cfg(unix)]
fn executable(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn executable(_: &std::path::Path) -> Result<(), String> {
    Err("Cargo launch-context capture is qualified on Linux only".into())
}
