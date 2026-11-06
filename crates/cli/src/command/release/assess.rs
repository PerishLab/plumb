use std::io::Read;
use std::path::Path;

pub(super) struct Assess<'a>(pub &'a Path);

impl Assess<'_> {
    pub fn run(&self) -> i32 {
        match self.read() {
            Ok(held) => match serde_json::to_string_pretty(&held) {
                Ok(body) => {
                    println!("{body}");
                    held.code()
                }
                Err(error) => {
                    eprintln!("plumb release assess: cannot encode assessment: {error}");
                    1
                }
            },
            Err(error) => {
                eprintln!("plumb release assess: {error}");
                1
            }
        }
    }

    fn read(&self) -> Result<plumb::semantics::Assessment, String> {
        let metadata = std::fs::symlink_metadata(self.0)
            .map_err(|error| format!("cannot inspect input: {error}"))?;
        if !metadata.is_file() || metadata.len() > plumb::semantics::Request::LIMIT as u64 {
            return Err("input requires a regular JSON file within 4 MiB".into());
        }
        let file =
            std::fs::File::open(self.0).map_err(|error| format!("cannot open input: {error}"))?;
        let mut bytes = Vec::new();
        file.take((plumb::semantics::Request::LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read input: {error}"))?;
        plumb::semantics::Request::parse(&bytes)?.assess()
    }
}
