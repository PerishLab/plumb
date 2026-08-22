use std::path::Path;
use std::process::Command;

pub struct Seat<'a>(pub &'a Path);

impl Seat<'_> {
    pub fn blind(&self, product: &str, built: Option<&str>) -> Option<String> {
        let built = built.filter(|_| product == "plumb")?;
        let head = self.reference("HEAD")?;
        if head == built || !self.ancestor(built, &head) {
            return None;
        }
        Some(format!(
            "running Plumb was built at {built}, before this Plumb tree at {head}; rebuild Plumb from the tree before asking Doctor to judge it"
        ))
    }

    fn reference(&self, name: &str) -> Option<String> {
        let output = self.git(["rev-parse", name]).output().ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn ancestor(&self, old: &str, new: &str) -> bool {
        self.git(["merge-base", "--is-ancestor", old, new])
            .status()
            .is_ok_and(|status| status.success())
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Command {
        let mut command = Command::new("git");
        command.arg("-C").arg(self.0).args(args);
        command
    }
}
