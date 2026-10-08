use serde_json::Value;
use std::collections::BTreeSet;

struct Object {
    keys: BTreeSet<String>,
    key: bool,
}

struct Reader<'a> {
    text: &'a str,
    index: usize,
    frames: Vec<Option<Object>>,
}

pub(super) fn read(bytes: &[u8], comments: bool) -> Result<Value, String> {
    if bytes.len() > 1_048_576 {
        return Err("static document exceeds its 1 MiB budget".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let text = text
        .lines()
        .filter(|line| !comments || !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    Reader {
        text: &text,
        index: 0,
        frames: Vec::new(),
    }
    .judge()?;
    Ok(value)
}

impl Reader<'_> {
    fn judge(&mut self) -> Result<(), String> {
        while let Some(byte) = self.text.as_bytes().get(self.index).copied() {
            match byte {
                b'{' => self.frames.push(Some(Object {
                    keys: BTreeSet::new(),
                    key: true,
                })),
                b'[' => self.frames.push(None),
                b'}' | b']' => {
                    self.frames.pop();
                }
                b',' => self.separator(),
                b'"' => self.string()?,
                _ => {}
            }
            self.index += 1;
        }
        Ok(())
    }

    fn separator(&mut self) {
        if let Some(Some(object)) = self.frames.last_mut() {
            object.key = true;
        }
    }

    fn string(&mut self) -> Result<(), String> {
        let start = self.index;
        self.index += 1;
        self.ending();
        let Some(Some(object)) = self.frames.last_mut() else {
            return Ok(());
        };
        if !object.key {
            return Ok(());
        }
        let key: String = serde_json::from_str(&self.text[start..=self.index])
            .map_err(|error| error.to_string())?;
        object.key = false;
        if !object.keys.insert(key.clone()) {
            return Err(format!("duplicate JSON field {key}"));
        }
        Ok(())
    }

    fn ending(&mut self) {
        while let Some(byte) = self.text.as_bytes().get(self.index) {
            match byte {
                b'"' => return,
                b'\\' => self.index += 2,
                _ => self.index += 1,
            }
        }
    }
}
