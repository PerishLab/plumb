const SEAT: &str = ">> \"$GITHUB_PATH\"";
const MANAGER: &str = "sh \"$manager\"";
const EMPTY: &str = "include\":[]";

pub fn refusals(path: &str, text: &str) -> Vec<String> {
    let mut held = Vec::new();
    if secreted(text) {
        held.push(format!(
            "{path} declares secrets under workflow_call, which this forge parses for inputs and outputs alone"
        ));
    }
    if text.matches(MANAGER).count() != text.matches(SEAT).count() {
        held.push(format!(
            "{path} installs a tool under the home seat without seating that seat on the job path"
        ));
    }
    if text.contains(EMPTY) {
        held.push(format!(
            "{path} renders an empty matrix, which this forge never creates, so every dependent blocks forever"
        ));
    }
    if path.ends_with(".release.yml") && pushed(text) {
        held.push(format!(
            "{path} starts a release from a push instead of waiting for an operator"
        ));
    }
    held
}

fn secreted(text: &str) -> bool {
    keyed(text, "  workflow_call:", "    secrets:")
}

fn pushed(text: &str) -> bool {
    keyed(text, "on:", "  push:")
}

fn keyed(text: &str, open: &str, wanted: &str) -> bool {
    let mut seen = false;
    for line in text.lines() {
        if line == open {
            seen = true;
            continue;
        }
        if seen && line == wanted {
            return true;
        }
        if seen && !line.starts_with(' ') && !line.is_empty() {
            return false;
        }
    }
    false
}
