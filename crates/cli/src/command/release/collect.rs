use plumb::semantics::collection::Collector;
use plumb::windtun::qualification::Remote;

pub(super) struct Collect<'a> {
    pub repository: &'a str,
    pub commit: &'a str,
}

impl Collect<'_> {
    pub fn run(&self) -> i32 {
        let report = Collector::new(Remote::default()).inspect(self.repository, self.commit);
        match serde_json::to_string_pretty(&report) {
            Ok(body) => {
                println!("{body}");
                match report.state {
                    "observed" => 0,
                    "unknown" => 3,
                    _ => 1,
                }
            }
            Err(error) => {
                eprintln!("plumb release collect: cannot encode observation: {error}");
                1
            }
        }
    }
}
