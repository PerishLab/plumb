mod value;

#[cfg(test)]
mod testing;

use crate::windtun::qualification::{Fault, Read};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use value::{Key, Value};

#[derive(Default)]
struct Materials {
    values: BTreeMap<Key, Value>,
    order: Vec<Key>,
    size: usize,
}

pub(super) struct Observation<'a> {
    reader: &'a dyn Read,
    materials: RefCell<Materials>,
    started: Instant,
}

impl<'a> Observation<'a> {
    const ENTRIES: usize = 1024;
    const BYTES: usize = 128 * 1024 * 1024;
    const TIME: Duration = Duration::from_secs(600);

    pub fn new(reader: &'a dyn Read) -> Self {
        Self {
            reader,
            materials: RefCell::default(),
            started: Instant::now(),
        }
    }

    fn budget(&self) -> Result<(), Fault> {
        if self.started.elapsed() >= Self::TIME {
            return Err(Fault::unknown("semantic observation time budget exhausted"));
        }
        Ok(())
    }

    fn remember(&self, key: Key, value: Value) -> Result<Value, Fault> {
        let mut materials = self.materials.borrow_mut();
        let size = key.size().saturating_add(value.size());
        if value.size() > key.limit() || size > Self::BYTES.saturating_sub(materials.size) {
            return Err(Fault::unknown("semantic observation byte budget exhausted"));
        }
        materials.size += size;
        materials.order.push(key.clone());
        materials.values.insert(key, value.clone());
        Ok(value)
    }

    fn read(&self, key: Key) -> Result<Value, Fault> {
        self.budget()?;
        let mut materials = self.materials.borrow_mut();
        if let Some(value) = materials.values.get(&key).cloned() {
            materials.order.retain(|held| held != &key);
            materials.order.push(key);
            return Ok(value);
        }
        if materials.values.len() >= Self::ENTRIES || key.size() > 4096 {
            return Err(Fault::unknown(
                "semantic observation entry budget exhausted",
            ));
        }
        drop(materials);
        let value = key.read(self.reader)?;
        self.budget()?;
        self.remember(key, value)
    }

    pub fn confirm(&self) -> Result<(), Fault> {
        let materials = self.materials.borrow();
        for key in &materials.order {
            self.budget()?;
            let repeated = key.read(self.reader)?;
            self.budget()?;
            if materials.values.get(key) != Some(&repeated) {
                return Err(key.moved());
            }
        }
        Ok(())
    }
}

impl Read for Observation<'_> {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        let Value::Bytes(raw) = self.read(Key::Bytes(url.into(), limit))? else {
            unreachable!()
        };
        Ok(raw)
    }

    fn optional(&self, url: &str, limit: usize) -> Result<Option<Vec<u8>>, Fault> {
        let Value::Optional(raw) = self.read(Key::Optional(url.into(), limit))? else {
            unreachable!()
        };
        Ok(raw)
    }

    fn image(&self, reference: &str) -> Result<String, Fault> {
        let Value::Image(value) = self.read(Key::Image(reference.into()))? else {
            unreachable!()
        };
        Ok(value)
    }
}
