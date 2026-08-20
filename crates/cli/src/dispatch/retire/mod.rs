mod cloud;
mod forge;
mod store;
mod target;

use clap::Args;
use cloud::{Bucket, Factory, Grant, Minted};
use plumb::config::Cascade as _;
use plumb::forgejo::Client;
use plumb::rig::Mint;
use std::path::PathBuf;
use store::{Seat, Store};
use target::Target;

const ADMIN: (&str, &str) = ("Workers R2 Storage Write", "com.cloudflare.api.account");
const ITEM: (&str, &str) = (
    "Workers R2 Storage Bucket Item Write",
    "com.cloudflare.edge.r2.bucket",
);
const MINUTES: u64 = 15;

#[derive(Args)]
pub struct Deed {
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    #[arg(long)]
    pub execute: bool,
    #[arg(long = "confirm-repo", default_value = "")]
    pub repo: String,
    #[arg(long = "confirm-bucket", default_value = "")]
    pub bucket: String,
    #[arg(long = "confirm-domain", default_value = "")]
    pub domain: String,
}

struct Names {
    writer: String,
    admin: String,
    item: String,
}

pub fn run(deed: Deed) -> i32 {
    match act(deed) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb retire: {error}");
            1
        }
    }
}

fn act(deed: Deed) -> Result<String, String> {
    let target = Target::read(&deed.root)?;
    let names = Names::of(&target.bucket);
    plan(&target, &names);
    if !deed.execute {
        return Ok("dry-run: no credentials read and no state changed".into());
    }
    target.confirm(&deed)?;
    let factory = open()?;
    factory.verify()?;
    let seen = factory.held()?;
    for stale in seen.iter().filter(|held| names.temporary(&held.name)) {
        factory.revoke(&stale.id)?;
        println!("temporary token: cleared stale {}", stale.name);
    }
    let writers = seen
        .iter()
        .filter(|held| held.name == names.writer)
        .collect::<Vec<_>>();
    if writers.len() > 1 {
        return Err(format!("duplicate persistent token {}", names.writer));
    }
    let writer = writers.first().map(|held| held.id.clone());
    let until = super::clock::ahead(MINUTES)?;
    let permits = (
        factory.permission(ADMIN.0, ADMIN.1)?,
        factory.permission(ITEM.0, ITEM.1)?,
    );
    let admin = factory.create(&Grant {
        name: names.admin.clone(),
        permission: permits.0,
        resource: format!("com.cloudflare.api.account.{}", factory.id()),
        expires: until.clone(),
    })?;
    let item = factory.create(&Grant {
        name: names.item.clone(),
        permission: permits.1,
        resource: format!(
            "com.cloudflare.edge.r2.bucket.{}_default_{}",
            factory.id(),
            target.bucket
        ),
        expires: until,
    });
    let outcome = item.as_ref().map_err(String::clone).and_then(|item| {
        sweep(
            &factory,
            &target,
            Seats {
                admin: &admin,
                item,
                writer,
            },
        )
    });
    revoke(&factory, item.ok().as_ref());
    revoke(&factory, Some(&admin));
    outcome
}

struct Seats<'a> {
    admin: &'a Minted,
    item: &'a Minted,
    writer: Option<String>,
}

fn sweep(factory: &Factory, target: &Target, seats: Seats<'_>) -> Result<String, String> {
    let bucket = Bucket::new(factory, seats.admin, &target.bucket);
    let live = bucket.live()?;
    let store = Store::new(
        &seats.item.id,
        &store::digest(seats.item.value()),
        &Seat {
            bucket: target.bucket.clone(),
            endpoint: format!("https://{}.r2.cloudflarestorage.com", factory.id()),
        },
    );
    let held = if live {
        store.inventory()?
    } else {
        store::Held {
            keys: Vec::new(),
            bytes: 0,
        }
    };
    println!(
        "r2 inventory: {} objects, {} bytes",
        held.keys.len(),
        held.bytes
    );

    let client = Client::new(target.remote.clone())?;
    let existed = forge::archive(&client)?;
    forge::purge(&client)?;
    match seats.writer {
        Some(id) => {
            factory.revoke(&id)?;
            println!("persistent token: revoked");
        }
        None => println!("persistent token: absent"),
    }
    if live {
        detach(&bucket, target)?;
        store.empty(&held.keys)?;
        bucket.erase()?;
        println!("bucket: deleted ({})", target.bucket);
    }
    if existed {
        forge::erase(&client)?;
        println!("forgejo: repository deleted ({})", target.repo());
    }
    escrow(target);
    Ok("retire: ok".into())
}

fn detach(bucket: &Bucket<'_>, target: &Target) -> Result<(), String> {
    let held = bucket.custom()?;
    if held.iter().any(|entry| entry.domain != target.domain) {
        return Err("bucket carries unexpected custom domains".into());
    }
    let Some(found) = held.first() else {
        println!("domain: absent ({})", target.domain);
        return Ok(());
    };
    if found.zone != target.zone {
        return Err(format!("custom domain belongs to zone {}", found.zone));
    }
    bucket.detach(&target.domain)?;
    println!("domain: detached ({})", target.domain);
    Ok(())
}

fn escrow(target: &Target) {
    let path = target.escrow();
    match std::fs::remove_file(&path) {
        Ok(()) => println!("escrow: deleted ({})", path.display()),
        Err(_) => println!("escrow: absent ({})", path.display()),
    }
}

fn revoke(factory: &Factory, minted: Option<&Minted>) {
    let Some(minted) = minted else {
        return;
    };
    match factory.revoke(&minted.id) {
        Ok(()) => println!("temporary token: revoked"),
        Err(fault) => eprintln!("temporary token: revocation failed: {fault}"),
    }
}

fn open() -> Result<Factory, String> {
    let held = Mint::default().merge(Mint::env("PLUMB_RETIRE").map_err(|error| error.to_string())?);
    if held.account.trim().is_empty() || held.token.trim().is_empty() {
        return Err("missing PLUMB_RETIRE_ACCOUNT, PLUMB_RETIRE_TOKEN".into());
    }
    if held.token.contains(['\r', '\n']) {
        return Err("PLUMB_RETIRE_TOKEN contains a line break".into());
    }
    Ok(Factory::new(
        held.account,
        held.api.trim_end_matches('/').to_string(),
        held.token,
    ))
}

fn plan(target: &Target, names: &Names) {
    println!("==> release retirement");
    println!("product: {}", target.product);
    println!("repo: {}", target.repo());
    println!("bucket: {}", target.bucket);
    println!("domain: {}", target.domain);
    println!("writer token: {}", names.writer);
    println!(
        "plan: inventory -> archive and purge credentials -> revoke writer -> detach domain -> empty and delete bucket -> delete repository -> remove escrow"
    );
}

impl Names {
    fn of(bucket: &str) -> Self {
        Self {
            writer: format!("w:{bucket}"),
            admin: format!("tmp:{bucket}"),
            item: format!("tmpw:{bucket}"),
        }
    }

    fn temporary(&self, name: &str) -> bool {
        name == self.admin || name == self.item
    }
}
