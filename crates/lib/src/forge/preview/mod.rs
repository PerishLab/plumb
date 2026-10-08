mod identity;
mod observation;
mod request;
mod result;

pub use observation::{Content, Deployment, Material, Provider, State};
pub use request::{Operation, Request, Source};
pub use result::{Observation, Outcome};

use serde::{Deserialize, Deserializer, Serialize};

fn nullable<'de, D, T>(reader: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(reader)
}

fn read<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    if bytes.len() > 65_536 {
        return Err("Preview protocol exceeds its 64 KiB budget".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut held = serde_json::to_value(value).map_err(|error| error.to_string())?;
    held.sort_all_objects();
    serde_json::to_vec(&held).map_err(|error| error.to_string())
}
