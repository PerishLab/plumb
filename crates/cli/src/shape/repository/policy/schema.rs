const LIMITS: [&str; 9] = [
    "block",
    "path",
    "param",
    "combination",
    "markup",
    "file",
    "fanout",
    "tuple",
    "schema",
];

const SYNTAX: [&str; 6] = ["embed", "environment", "lock", "process", "style", "test"];

const EXEMPT: [&str; 17] = [
    "block",
    "burr",
    "combination",
    "comment",
    "dispatch",
    "exec",
    "fanout",
    "file",
    "ignore",
    "markup",
    "param",
    "path",
    "receiver",
    "schema",
    "shadow",
    "tempdir",
    "tuple",
];

const SEALED: [&str; 5] = ["ban", "coverage", "grant", "reach", "word"];

pub(crate) fn validate(overlay: &toml::Table) -> Result<(), String> {
    for (key, value) in overlay {
        let field = Field(value);
        match key.as_str() {
            "scan" => field.lists("scan", &["include", "exclude"])?,
            "module" => field.lists("module", &["roots"])?,
            "limit" => field.limit()?,
            "comment" => field.flag("comment", "allow")?,
            "boundary" => field.boundaries()?,
            "grant" => field.rules("grant")?,
            "ban" => field.rules("ban")?,
            _ => return Err(format!("unknown key `{key}`")),
        }
    }
    Ok(())
}

struct Field<'a>(&'a toml::Value);

impl<'a> Field<'a> {
    fn table(&self, name: &str) -> Result<&'a toml::Table, String> {
        self.0
            .as_table()
            .ok_or_else(|| format!("{name} must be a table"))
    }

    fn strings(&self, name: &str) -> Result<Vec<&'a str>, String> {
        let items = self
            .0
            .as_array()
            .ok_or_else(|| format!("{name} must be an array of strings"))?;
        items
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or_else(|| format!("{name} must be an array of strings"))
            })
            .collect()
    }

    fn globs(&self, name: &str) -> Result<Vec<&'a str>, String> {
        let items = self.strings(name)?;
        for pattern in &items {
            glob(pattern)?;
        }
        Ok(items)
    }

    fn tables(&self, name: &str) -> Result<Vec<&'a toml::Table>, String> {
        let items = self
            .0
            .as_array()
            .ok_or_else(|| format!("{name} must be an array of tables"))?;
        items
            .iter()
            .map(|entry| {
                entry
                    .as_table()
                    .ok_or_else(|| format!("{name} must be an array of tables"))
            })
            .collect()
    }

    fn lists(&self, section: &str, allowed: &[&str]) -> Result<(), String> {
        for (key, entry) in self.table(section)? {
            if !allowed.contains(&key.as_str()) {
                return Err(format!("unknown key `{section}.{key}`"));
            }
            Field(entry).globs(&format!("{section}.{key}"))?;
        }
        Ok(())
    }

    fn limit(&self) -> Result<(), String> {
        for (key, entry) in self.table("limit")? {
            if !LIMITS.contains(&key.as_str()) {
                return Err(format!("unknown key `limit.{key}`"));
            }
            if entry.as_integer().is_none_or(|value| value < 0) {
                return Err(format!("limit.{key} must be a non-negative integer"));
            }
        }
        Ok(())
    }

    fn flag(&self, section: &str, allowed: &str) -> Result<(), String> {
        for (key, entry) in self.table(section)? {
            if key != allowed {
                return Err(format!("unknown key `{section}.{key}`"));
            }
            if !entry.is_bool() {
                return Err(format!("{section}.{key} must be a boolean"));
            }
        }
        Ok(())
    }

    fn boundaries(&self) -> Result<(), String> {
        for entry in self.tables("boundary")? {
            Entry(entry).boundary()?;
        }
        Ok(())
    }

    fn rules(&self, name: &str) -> Result<(), String> {
        for entry in self.tables(name)? {
            Entry(entry).rule(name)?;
        }
        Ok(())
    }
}

struct Entry<'a>(&'a toml::Table);

impl<'a> Entry<'a> {
    fn known(&self, section: &str, allowed: &[&str]) -> Result<(), String> {
        for key in self.0.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(format!("unknown key `{section}.{key}`"));
            }
        }
        Ok(())
    }

    fn text(&self, section: &str, key: &str) -> Result<Option<&'a str>, String> {
        match self.0.get(key) {
            None => Ok(None),
            Some(value) => value
                .as_str()
                .map(Some)
                .ok_or_else(|| format!("{section}.{key} must be a string")),
        }
    }

    fn boundary(&self) -> Result<(), String> {
        self.known("boundary", &["paths", "allow", "note"])?;
        let paths = match self.0.get("paths") {
            Some(value) => Field(value).globs("boundary.paths")?,
            None => Vec::new(),
        };
        if paths.is_empty() {
            return Err("boundary paths must not be empty".to_string());
        }
        let note = self.text("boundary", "note")?.unwrap_or("");
        if note.trim().is_empty() {
            return Err("boundary note must not be empty".to_string());
        }
        let laws = match self.0.get("allow") {
            Some(value) => Field(value).strings("boundary.allow")?,
            None => Vec::new(),
        };
        if laws.is_empty() {
            return Err("boundary allow must not be empty".to_string());
        }
        for name in laws {
            allowed(name)?;
        }
        Ok(())
    }

    fn rule(&self, name: &str) -> Result<(), String> {
        self.known(name, &["syntax", "paths"])?;
        let kind = self.text(name, "syntax")?.unwrap_or("");
        if !SYNTAX.contains(&kind) {
            return Err(format!("unknown {name} syntax `{kind}`"));
        }
        let paths = match self.0.get("paths") {
            Some(value) => Field(value).globs(&format!("{name}.paths"))?,
            None => Vec::new(),
        };
        if paths.is_empty() {
            return Err(format!("{name} `{kind}` paths must not be empty"));
        }
        Ok(())
    }
}

fn allowed(name: &str) -> Result<(), String> {
    if EXEMPT.contains(&name) {
        return Ok(());
    }
    if SEALED.contains(&name) {
        return Err(format!("boundary law `{name}` admits no exemption"));
    }
    Err(format!("unknown boundary law `{name}`"))
}

fn glob(pattern: &str) -> Result<(), String> {
    if pattern.trim().is_empty() {
        return Err("glob must not be empty".to_string());
    }
    if pattern.contains('\\') {
        return Err(format!("glob `{pattern}` must use forward slashes"));
    }
    for part in pattern.trim_matches('/').split('/') {
        if part.is_empty() || matches!(part, "." | "..") {
            return Err(format!("invalid glob segment in `{pattern}`"));
        }
        if part.contains("**") && part != "**" {
            return Err(format!(
                "recursive wildcard must occupy a whole segment in `{pattern}`"
            ));
        }
        if part != "**" && part.matches('*').count() > 1 {
            return Err(format!(
                "glob segment has multiple wildcards in `{pattern}`"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn retired() {
        let cases = [
            (
                "[[boundary]]\npaths = [\"a.rs\"]\nallow = [\"word\"]\nnote = \"why\"\n",
                "boundary law `word` admits no exemption",
            ),
            ("[word]\n", "unknown key `word`"),
            ("[word]\nsingle = true\n", "unknown key `word`"),
            ("[word]\nsingle = false\n", "unknown key `word`"),
            ("[vocabulary]\n", "unknown key `vocabulary`"),
            (
                "[[vocabulary.term]]\nname = \"vite_config\"\ndescription = \"fixture\"\n",
                "unknown key `vocabulary`",
            ),
        ];
        for (input, expected) in cases {
            let table: toml::Table = input.parse().expect("table");
            let error = validate(&table).expect_err("retired schema must fail");
            assert_eq!(error, expected, "{input}");
        }
    }

    #[test]
    fn valid() {
        let sparse: toml::Table = "[[boundary]]\npaths = [\"a.rs\"]\nallow = [\"path\"]\nnote = \"ok\"\n\n[[grant]]\nsyntax = \"lock\"\npaths = [\"a.rs\"]\n"
            .parse()
            .expect("table");
        assert!(validate(&toml::Table::new()).is_ok());
        assert!(validate(&sparse).is_ok());
    }
}
