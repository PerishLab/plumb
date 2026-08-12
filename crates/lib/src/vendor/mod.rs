pub mod cloudflare;
pub mod forgejo;
mod request;

pub use request::{Response, public, send};
