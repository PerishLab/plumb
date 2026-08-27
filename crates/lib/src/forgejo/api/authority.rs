use super::Client;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub id: u64,
    pub name: String,
    pub scopes: BTreeSet<String>,
    pub last: String,
}

pub struct Minted {
    pub token: Token,
    pub secret: String,
}

impl Client {
    pub fn tokens(&self, account: &str) -> Result<Vec<Token>, String> {
        let reply = self.reply(&["admin", "token", "list", account])?;
        let values = reply
            .value
            .as_array()
            .ok_or_else(|| "forgejo admin token list is not an array".to_string())?;
        values.iter().map(token).collect()
    }

    pub fn mint(&self, account: &str, name: &str) -> Result<Minted, String> {
        let result = (|| {
            let wire = scopes().into_iter().collect::<Vec<_>>().join(",");
            let reply =
                self.reply(&["admin", "token", "create", account, name, "--scopes", &wire])?;
            let secret = reply
                .secret()
                .map(|held| held.expose().to_string())
                .ok_or_else(|| "forgejo token issuer returned no guarded secret".to_string())?;
            if !secret.is_ascii() || secret.len() <= 8 || secret.chars().any(char::is_whitespace) {
                return Err("forgejo token issuer returned an invalid guarded secret".into());
            }
            let matches = self
                .tokens(account)?
                .into_iter()
                .filter(|held| held.name == name)
                .collect::<Vec<_>>();
            let [token] = matches.as_slice() else {
                return Err(format!(
                    "expected exactly one minted registry token {name}, saw {}",
                    matches.len()
                ));
            };
            let last = &secret[secret.len() - 8..];
            if token.scopes != scopes() || token.last != last {
                return Err(
                    "minted registry token did not verify against its guarded secret".into(),
                );
            }
            self.verify(account, &secret)?;
            Ok(Minted {
                token: token.clone(),
                secret,
            })
        })();
        match result {
            Ok(minted) => Ok(minted),
            Err(error) => match self.cleanup(account, name) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!(
                    "{error}; registry capability rollback also failed: {cleanup}"
                )),
            },
        }
    }

    pub fn revoke(&self, account: &str, token: &Token) -> Result<(), String> {
        self.reply(&[
            "admin",
            "token",
            "delete",
            account,
            &token.name,
            &token.id.to_string(),
        ])?;
        if self
            .tokens(account)?
            .iter()
            .any(|held| held.id == token.id && held.name == token.name)
        {
            Err(format!(
                "forgejo token {} remained after revocation",
                token.name
            ))
        } else {
            Ok(())
        }
    }

    pub fn verify(&self, account: &str, secret: &str) -> Result<(), String> {
        let mut vars = self.vars.clone();
        vars.remove("FORGEJO_TOKEN_FILE");
        vars.insert("FORGEJO_TOKEN".into(), secret.to_string());
        let mut argv = vec![
            "--repo".to_string(),
            format!("{}/{}", self.remote.owner, self.remote.repo),
        ];
        argv.extend(["user".to_string(), "show".to_string()]);
        let reply = runseal::tool::call("forgejo", &argv, &vars).map_err(|error| {
            format!(
                "forgejo refused user show on {}/{}: {error}",
                self.remote.owner, self.remote.repo
            )
        })?;
        let login = reply
            .value
            .get("login")
            .and_then(Value::as_str)
            .unwrap_or("");
        if login == account {
            Ok(())
        } else {
            Err("forgejo minted-token identity disagrees".into())
        }
    }

    fn cleanup(&self, account: &str, name: &str) -> Result<(), String> {
        let tokens = self
            .tokens(account)?
            .into_iter()
            .filter(|token| token.name == name)
            .collect::<Vec<_>>();
        for token in &tokens {
            self.revoke(account, token)?;
        }
        Ok(())
    }
}

pub fn scopes() -> BTreeSet<String> {
    ["public-only", "read:user", "write:package"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub fn token(value: &Value) -> Result<Token, String> {
    let id = value
        .get("id")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or_else(|| "forgejo admin token has no positive ID".to_string())?;
    let name = field(value, "name")?;
    let last = field(value, "token_last_eight")?;
    let scopes = value
        .get("scopes")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("forgejo admin token {name} has no scopes"))?
        .iter()
        .map(|scope| {
            scope
                .as_str()
                .filter(|held| !held.is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("forgejo admin token {name} has an invalid scope"))
        })
        .collect::<Result<_, _>>()?;
    if last.len() != 8 {
        return Err(format!("forgejo admin token {name} has invalid last-eight"));
    }
    Ok(Token {
        id,
        name,
        scopes,
        last,
    })
}

fn field(value: &Value, name: &str) -> Result<String, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|held| !held.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("forgejo admin token has no {name}"))
}
