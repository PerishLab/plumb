pub(super) struct Template<'a>(pub(super) &'a str);

impl Template<'_> {
    pub(super) fn env(&self) -> Vec<String> {
        let mut found = Vec::new();
        for line in self.0.lines() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            if key == "name" && legacy(value) {
                found.push(value.to_string());
            }
        }
        found
    }

    pub(super) fn mismatch(&self) -> bool {
        let lines: Vec<&str> = self.0.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            if key != "securityContext" || !value.is_empty() {
                continue;
            }
            if block(&lines[index + 1..], indent(line)) {
                return true;
            }
        }
        false
    }

    pub(super) fn configmap(&self) -> bool {
        documents(self.0).any(|doc| {
            let mut kind = false;
            let mut key = false;
            for line in doc.lines() {
                if let Some((name, value)) = entry(line) {
                    kind |= name == "kind" && value == "ConfigMap";
                    key |= name == "client.json";
                }
            }
            kind && key
        })
    }

    pub(super) fn mount(&self) -> bool {
        let lines: Vec<&str> = self.0.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            if key != "volumeMounts" || !value.is_empty() {
                continue;
            }
            if mounted(&lines[index + 1..], indent(line)) {
                return true;
            }
        }
        false
    }

    pub(super) fn checksum(&self) -> bool {
        self.0.lines().any(|line| {
            entry(line).is_some_and(|(key, value)| key == "checksum/client" && !value.is_empty())
        })
    }

    pub(super) fn workload(&self) -> bool {
        documents(self.0).any(pod)
    }

    pub(super) fn unsecret(&self) -> bool {
        documents(self.0).any(|doc| pod(doc) && !secret(doc))
    }
}

pub(super) fn declared(values: &str) -> bool {
    values.lines().any(|line| {
        indent(line) == 0 && entry(line).is_some_and(|(key, _)| key == "imagePullSecrets")
    })
}

fn pod(doc: &str) -> bool {
    let mut kind = false;
    let mut containers = false;
    for line in doc.lines() {
        if let Some((key, value)) = entry(line) {
            kind |= key == "kind"
                && matches!(
                    value,
                    "Deployment" | "StatefulSet" | "Job" | "CronJob" | "DaemonSet"
                );
            containers |= key == "containers" && value.is_empty();
        }
    }
    kind && containers
}

fn secret(doc: &str) -> bool {
    doc.lines()
        .any(|line| entry(line).is_some_and(|(key, _)| key == "imagePullSecrets"))
}

pub(super) fn defined(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let trimmed = strip(lines[index]).trim();
        if !opens(trimmed) {
            index += 1;
            continue;
        }
        let mut depth = 1usize;
        let mut group = false;
        let mut policy = false;
        index += 1;
        while index < lines.len() && depth > 0 {
            let line = lines[index];
            let body = strip(line).trim();
            depth = step(depth, body);
            if depth > 0
                && let Some((key, value)) = entry(line)
            {
                group |= key == "fsGroup";
                policy |= key == "fsGroupChangePolicy" && value == "OnRootMismatch";
            }
            index += 1;
        }
        if group && !policy {
            return true;
        }
    }
    false
}

fn opens(trimmed: &str) -> bool {
    keyword(trimmed) == Some("define")
}

fn step(depth: usize, trimmed: &str) -> usize {
    match keyword(trimmed) {
        Some("if" | "with" | "range" | "define" | "block") => depth.saturating_add(1),
        Some("end") => depth.saturating_sub(1),
        _ => depth,
    }
}

fn keyword(trimmed: &str) -> Option<&str> {
    let inner = trimmed
        .strip_prefix("{{-")
        .or_else(|| trimmed.strip_prefix("{{"))?
        .strip_suffix("-}}")
        .or_else(|| trimmed.strip_suffix("}}"))?
        .trim();
    inner.split_whitespace().next()
}

fn documents(text: &str) -> impl Iterator<Item = &str> {
    text.split("\n---")
}

fn block(lines: &[&str], base: usize) -> bool {
    let mut group = false;
    let mut policy = false;
    for line in lines {
        let trimmed = strip(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        let depth = indent(line);
        if directive(trimmed) {
            if depth <= base && keyword(trimmed) == Some("end") {
                break;
            }
            continue;
        }
        if depth <= base {
            break;
        }
        if let Some((key, value)) = entry(line) {
            group |= key == "fsGroup";
            policy |= key == "fsGroupChangePolicy" && value == "OnRootMismatch";
        }
    }
    group && !policy
}

fn mounted(lines: &[&str], base: usize) -> bool {
    let mut path = false;
    let mut sub = false;
    let mut readonly = false;
    for line in lines {
        let trimmed = strip(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        let depth = indent(line);
        if directive(trimmed) {
            if depth <= base && keyword(trimmed) == Some("end") {
                break;
            }
            continue;
        }
        if depth <= base {
            break;
        }
        if trimmed.starts_with("- ") {
            if path && sub && readonly {
                return true;
            }
            path = false;
            sub = false;
            readonly = false;
        }
        if let Some((key, value)) = entry(line) {
            path |= key == "mountPath" && value == "/web/dist/client.json";
            sub |= key == "subPath" && value == "client.json";
            readonly |= key == "readOnly" && value == "true";
        }
    }
    path && sub && readonly
}

fn legacy(value: &str) -> bool {
    value.starts_with("API_")
        && value.len() > 4
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn entry(line: &str) -> Option<(&str, &str)> {
    let trimmed = strip(line).trim();
    if trimmed.is_empty() || directive(trimmed) {
        return None;
    }
    let body = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    let (key, value) = body.split_once(':')?;
    let key = key.trim();
    let value = value.trim().trim_matches(['"', '\'']);
    (!key.is_empty()).then_some((key, value))
}

fn strip(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return "";
    }
    line.split(" #").next().unwrap_or(line)
}

fn directive(trimmed: &str) -> bool {
    trimmed.starts_with("{{") && trimmed.ends_with("}}")
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
