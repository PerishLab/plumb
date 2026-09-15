use super::{command, success, text};
use std::path::Path;

pub(super) struct Seat<'a>(pub(super) &'a Path);

impl Seat<'_> {
    pub(super) fn retain(&self, sources: &[String]) -> Result<(), String> {
        for source in sources {
            super::super::value::commit(source)?;
            let actual = text(
                "resolve pick source",
                command(self.0, ["rev-parse", &format!("{source}^{{commit}}")])?,
            )?;
            if actual != *source {
                return Err(format!(
                    "pick source {source} does not name an exact commit"
                ));
            }
            let name = reference(source);
            if !self.standing(source)? {
                let pushed = command(
                    self.0,
                    [
                        "push",
                        &format!("--force-with-lease={name}:"),
                        "origin",
                        &format!("{source}:{name}"),
                    ],
                )?;
                if !pushed.status.success() && !self.standing(source)? {
                    success("retain pick source evidence", pushed)?;
                }
            }
            self.load(source)?;
        }
        Ok(())
    }

    pub(super) fn available(&self, sources: &[String]) -> Result<(), String> {
        for source in sources {
            if !command(self.0, ["cat-file", "-e", &format!("{source}^{{commit}}")])?
                .status
                .success()
            {
                self.load(source)?;
            }
        }
        Ok(())
    }

    pub(super) fn load(&self, source: &str) -> Result<(), String> {
        if !self.standing(source)? {
            return Err(format!(
                "pick source {source} has no retained remote evidence; repeat its original version pick from the preserved source checkout"
            ));
        }
        let name = reference(source);
        success(
            "fetch pick source evidence",
            command(self.0, ["fetch", "--no-tags", "origin", &name])?,
        )?;
        let actual = text(
            "read back pick source evidence",
            command(self.0, ["rev-parse", "FETCH_HEAD^{commit}"])?,
        )?;
        if actual != source {
            return Err(format!(
                "pick source evidence {name} changed while fetching"
            ));
        }
        Ok(())
    }

    fn standing(&self, source: &str) -> Result<bool, String> {
        super::super::value::commit(source)?;
        let name = reference(source);
        let held = text(
            "inspect remote pick source evidence",
            command(self.0, ["ls-remote", "--refs", "origin", &name])?,
        )?;
        if held.is_empty() {
            return Ok(false);
        }
        if held != format!("{source}\t{name}") {
            return Err(format!(
                "pick source evidence {name} disagrees with {source}"
            ));
        }
        Ok(true)
    }
}

fn reference(source: &str) -> String {
    format!("refs/plumb/picks/{source}")
}
