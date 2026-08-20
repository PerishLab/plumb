use std::path::Path;

const GUARDS: [&str; 2] = [
    ".forgejo/workflows/guard.yml",
    ".github/workflows/quality.yml",
];

pub struct Guard {
    pub lanes: Vec<(String, String)>,
    pub source: String,
}

pub struct Operator<'a>(pub &'a Path);

impl Operator<'_> {
    pub fn guard(&self) -> Guard {
        let mut lanes = Vec::new();
        for seat in GUARDS {
            let Ok(source) = std::fs::read_to_string(self.0.join(seat)) else {
                continue;
            };
            lanes.push((seat.to_string(), source.replace("\r\n", "\n")));
        }
        let source = lanes
            .iter()
            .map(|(_, source)| source.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        Guard { lanes, source }
    }
}
