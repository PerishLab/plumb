use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;

pub(super) struct Unique(pub Value);

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> std::result::Result<Self, D::Error> {
        reader.deserialize_any(Self(Value::Null))
    }
}

impl<'de> Visitor<'de> for Unique {
    type Value = Self;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded JSON with unique keys and integral protocol numbers")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Self, E> {
        Ok(Self(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Self, E> {
        Ok(Self(Value::Number(Number::from(value))))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Self, E> {
        Ok(Self(Value::Number(Number::from(value))))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Self, E> {
        Ok(Self(Value::String(value.into())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<Self, E> {
        Ok(Self(Value::String(value)))
    }

    fn visit_unit<E: de::Error>(self) -> std::result::Result<Self, E> {
        Ok(Self(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> std::result::Result<Self, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element::<Self>()? {
            values.push(value.0);
        }
        Ok(Self(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> std::result::Result<Self, A::Error> {
        let mut values = Map::new();
        while let Some((key, value)) = access.next_entry::<String, Self>()? {
            if values.insert(key, value.0).is_some() {
                return Err(de::Error::custom("duplicate JSON key"));
            }
        }
        Ok(Self(Value::Object(values)))
    }
}
