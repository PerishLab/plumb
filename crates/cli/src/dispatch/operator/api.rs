use super::request::{Response, failure, send};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct Remote {
    pub scheme: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
}

pub struct Client {
    pub(super) remote: Remote,
    token: String,
}

pub struct Pull {
    pub number: u64,
}

pub struct State {
    pub state: String,
    pub count: usize,
}

impl Client {
    pub fn new(remote: Remote) -> Result<Self, String> {
        let token = super::token::read(&remote)?;
        if token.contains(['\r', '\n']) {
            return Err("forgejo token contains a line break".into());
        }
        Ok(Self { remote, token })
    }

    pub fn branch(&self, name: &str) -> Result<Option<Value>, String> {
        let response = self.request("GET", &format!("/branches/{}", segment(name)), None)?;
        match response.status {
            200 if response.value.is_object() => Ok(Some(response.value)),
            404 => Ok(None),
            status => Err(failure("fetching branch", status, &response.value)),
        }
    }

    pub fn create(&self, name: &str, from: &str) -> Result<(), String> {
        if self.branch(name)?.is_some() {
            return Ok(());
        }
        let response = self.request(
            "POST",
            "/branches",
            Some(json!({"new_branch_name": name, "old_branch_name": from})),
        )?;
        if response.status == 201 {
            Ok(())
        } else {
            Err(failure(
                "creating release branch",
                response.status,
                &response.value,
            ))
        }
    }

    pub fn protect(&self, name: &str, mode: &str) -> Result<(), String> {
        let username = self.user()?;
        let route = format!("/branch_protections/{}", segment(name));
        let existing = self.request("GET", &route, None)?;
        if ![200, 404].contains(&existing.status) {
            return Err(failure(
                "fetching branch protection",
                existing.status,
                &existing.value,
            ));
        }
        let policy = super::protection::policy(name, mode, &username);
        let response = if existing.status == 404 {
            self.request("POST", "/branch_protections", Some(policy.clone()))?
        } else {
            self.request("PATCH", &route, Some(policy.clone()))?
        };
        let wanted = if existing.status == 404 { 201 } else { 200 };
        if response.status != wanted {
            return Err(failure(
                "setting branch protection",
                response.status,
                &response.value,
            ));
        }
        super::protection::verify(name, &policy, &response.value)
    }

    pub fn find(&self, head: &str) -> Result<Option<Pull>, String> {
        let response = self.request("GET", "/pulls?state=open&limit=50", None)?;
        if response.status != 200 {
            return Err(failure("listing pulls", response.status, &response.value));
        }
        Ok(response
            .value
            .as_array()
            .into_iter()
            .flatten()
            .find(|pull| {
                pull.pointer("/head/ref").and_then(Value::as_str) == Some(head)
                    && pull.pointer("/base/ref").and_then(Value::as_str) == Some("main")
            })
            .and_then(pull))
    }

    pub fn pull(&self, head: &str, version: &str, body: &str) -> Result<Pull, String> {
        let response = self.request(
            "POST",
            "/pulls",
            Some(json!({
                "base": "main",
                "head": head,
                "title": format!("Packport {version}"),
                "body": body
            })),
        )?;
        if response.status == 201 {
            pull(&response.value).ok_or_else(|| "created pull has no number".to_string())
        } else {
            Err(failure(
                "creating packport pull",
                response.status,
                &response.value,
            ))
        }
    }

    pub fn context(&self, commit: &str, context: &str) -> Result<State, String> {
        let route = format!("/commits/{commit}/status");
        let response = self.request("GET", &route, None)?;
        if response.status != 200 {
            return Err(failure(
                "fetching commit status",
                response.status,
                &response.value,
            ));
        }
        let mut found = response
            .value
            .get("statuses")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|status| status.get("context").and_then(Value::as_str) == Some(context))
            .collect::<Vec<_>>();
        found.sort_by_key(|status| {
            status
                .get("updated_at")
                .or_else(|| status.get("created_at"))
                .and_then(Value::as_str)
                .unwrap_or("")
        });
        Ok(State {
            state: found
                .last()
                .and_then(|status| status.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            count: found.len(),
        })
    }

    pub fn merge(&self, pull: u64, head: &str) -> Result<(), String> {
        let body = json!({
            "Do": "merge",
            "delete_branch_after_merge": false,
            "head_commit_id": head
        });
        let mut last = String::new();
        for turn in 1..=6 {
            let response =
                self.request("POST", &format!("/pulls/{pull}/merge"), Some(body.clone()))?;
            if [200, 204].contains(&response.status) {
                return Ok(());
            }
            last = failure("merging packport pull", response.status, &response.value);
            if ![405, 409].contains(&response.status) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(turn));
        }
        Err(last)
    }

    fn user(&self) -> Result<String, String> {
        let response = send(
            &format!("{}://{}/api/v1/user", self.remote.scheme, self.remote.host),
            "GET",
            None,
            Some(&self.token),
        )?;
        let login = response
            .value
            .get("login")
            .and_then(Value::as_str)
            .unwrap_or("");
        if response.status == 200 && !login.is_empty() {
            Ok(login.to_string())
        } else {
            Err(failure(
                "fetching current user",
                response.status,
                &response.value,
            ))
        }
    }

    pub(super) fn request(
        &self,
        method: &str,
        route: &str,
        body: Option<Value>,
    ) -> Result<Response, String> {
        send(
            &format!(
                "{}://{}/api/v1/repos/{}/{}{}",
                self.remote.scheme, self.remote.host, self.remote.owner, self.remote.repo, route
            ),
            method,
            body,
            Some(&self.token),
        )
    }
}

pub fn public(url: &str) -> Result<Value, String> {
    super::request::public(url)
}

fn pull(value: &Value) -> Option<Pull> {
    value
        .get("number")
        .and_then(Value::as_u64)
        .map(|number| Pull { number })
}

fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|held| match held {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (held as char).to_string()
            }
            _ => format!("%{held:02X}"),
        })
        .collect()
}
