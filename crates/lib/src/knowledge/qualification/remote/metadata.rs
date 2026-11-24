use super::super::{Fault, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use runseal::github::{Blob, Client, Error, Invocation, Listing, Receipt, Reference};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn read(client: &Client, path: &str, limit: usize) -> Result<Vec<u8>> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let segments: Vec<_> = route.split('/').collect();
    let ["repos", owner, name, rest @ ..] = segments.as_slice() else {
        return Err(Fault::refused("unsupported GitHub metadata coordinate"));
    };
    let source = Repository {
        client,
        owner,
        name,
    };
    let value = source.observe(rest, query).map_err(|fault| {
        let digest = Sha256::digest(path.as_bytes());
        Fault {
            reason: format!("GitHub metadata [request sha256:{digest:x}]: {fault}"),
            ..fault
        }
    })?;
    encode(&value, limit)
}

struct Repository<'a> {
    client: &'a Client,
    owner: &'a str,
    name: &'a str,
}

impl Repository<'_> {
    fn observe(&self, route: &[&str], query: &str) -> Result<Value> {
        match (route, query) {
            (["git", rest @ ..], query) => self.git(rest, query),
            (["actions", "runs", run, "attempts", attempt, rest @ ..], query) => {
                self.attempt(run, attempt, rest, query)
            }
            (["contents", path @ ..], query) => self.content(path, query),
            (["tags"], query) => self.tags(query),
            ([], "") => value(self.client.repository(self.owner, self.name)),
            (["branches", branch], "") => value(self.client.branch(self.owner, self.name, branch)),
            (["compare", pair], "") => self.lineage(pair),
            (["issues", "comments", id], "") => self.comment(id),
            (["collaborators", login, "permission"], "") => {
                value(self.client.permission(self.owner, self.name, login))
            }
            _ => Err(Fault::refused("unsupported GitHub metadata route")),
        }
    }

    fn git(&self, route: &[&str], query: &str) -> Result<Value> {
        let (client, owner, name) = (self.client, self.owner, self.name);
        match (route, query) {
            (["commits", sha], "") => value(client.commit(owner, name, sha)),
            (["ref", "tags", tag], "") => {
                value(client.reference(owner, name, &Reference::Tag((*tag).into())))
            }
            (["tags", sha], "") => value(client.annotation(owner, name, sha)),
            (["trees", sha], "recursive=1") => value(client.tree(owner, name, sha, true)),
            (["blobs", sha], "") => client
                .blob(owner, name, sha)
                .map_err(provider)
                .map(|r| blob(r.value)),
            _ => Err(Fault::refused("unsupported GitHub Git metadata route")),
        }
    }

    fn attempt(&self, run: &str, attempt: &str, route: &[&str], query: &str) -> Result<Value> {
        let run = number(run)?;
        let attempt = number(attempt)?;
        match (route, query) {
            ([], "") => value(self.client.workflow(
                self.owner,
                self.name,
                Invocation::new(run, Some(attempt)).map_err(provider)?,
            )),
            (["jobs"], "per_page=100") => {
                let input = Listing::new(run, attempt, 1, 100).map_err(provider)?;
                value(self.client.jobs(self.owner, self.name, input))
            }
            _ => Err(Fault::refused("unsupported GitHub Actions metadata route")),
        }
    }

    fn content(&self, path: &[&str], query: &str) -> Result<Value> {
        let commit = query
            .strip_prefix("ref=")
            .ok_or_else(|| Fault::refused("source content requires an exact commit"))?;
        let path = path.join("/");
        let held = self
            .client
            .contents(self.owner, self.name, commit, &path)
            .map_err(provider)?
            .value;
        let mut body = blob(held.blob);
        body["type"] = json!("file");
        body["path"] = json!(held.path);
        Ok(body)
    }

    fn tags(&self, query: &str) -> Result<Value> {
        let page = query
            .strip_prefix("per_page=100&page=")
            .ok_or_else(|| Fault::refused("unsupported native tag page"))?;
        let receipt = self
            .client
            .tags(self.owner, self.name, number(page)?, 100)
            .map_err(provider)?;
        serde_json::to_value(receipt.value.rows)
            .map_err(|_| Fault::refused("cannot encode native tag page"))
    }

    fn lineage(&self, pair: &str) -> Result<Value> {
        let (base, head) = pair
            .split_once("...")
            .ok_or_else(|| Fault::refused("invalid GitHub comparison"))?;
        value(self.client.ancestry((self.owner, self.name), base, head))
    }

    fn comment(&self, id: &str) -> Result<Value> {
        let found = self
            .client
            .lookup(self.owner, self.name, number(id)?)
            .map_err(provider)?
            .value;
        let mut body = serde_json::to_value(found.entry)
            .map_err(|_| Fault::refused("cannot encode comment evidence"))?;
        body["user"] = json!(found.author);
        body["updated_at"] = json!(found.updated);
        Ok(body)
    }
}

fn value<T: Serialize>(receipt: std::result::Result<Receipt<T>, Error>) -> Result<Value> {
    serde_json::to_value(receipt.map_err(provider)?.value)
        .map_err(|_| Fault::refused("cannot encode GitHub metadata"))
}

fn provider(error: Error) -> Fault {
    Fault::unknown(crate::config::github::diagnostic(error))
}

fn number<T: std::str::FromStr>(raw: &str) -> Result<T> {
    raw.parse()
        .map_err(|_| Fault::refused("invalid GitHub metadata number"))
}

fn blob(blob: Blob) -> Value {
    json!({"sha":blob.sha,"size":blob.size,"encoding":"base64","content":STANDARD.encode(blob.bytes)})
}

fn encode(value: &Value, limit: usize) -> Result<Vec<u8>> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| Fault::refused("cannot encode GitHub metadata"))?;
    if bytes.len() > limit {
        return Err(Fault::refused("GitHub metadata output exceeds bounds"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bounds() {
        let value = serde_json::json!({"kind":"bounded"});
        let size = serde_json::to_vec(&value).unwrap().len();
        assert!(super::encode(&value, size).is_ok());
        assert!(super::encode(&value, size - 1).is_err());
    }
}
