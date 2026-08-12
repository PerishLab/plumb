# Migrating to Plumb v0.18.18

Existing repositories require no migration. `[release.retire]` is optional and
every manifest without it parses unchanged.

A product that should be retirable declares its two irreducible facts:

```toml
[release.retire]
bucket = "perish-<product>-releases"
zone = "<32-character lowercase hex zone id>"
```

Declaring nothing is the safe default and means the product cannot be retired
by this command at all. The custom domain is derived from the release
authority, so do not restate it.

Retirement needs a Cloudflare token factory in the environment, provisioned
outside Plumb:

```sh
PLUMB_RETIRE_ACCOUNT=<account id>
PLUMB_RETIRE_TOKEN=<factory token>
```

`PLUMB_RETIRE_API` defaults to the Cloudflare v4 endpoint. Plumb consumes this
credential and never mints one; provisioning it remains the resource owner's
work.

Read the dry run before acting. It reads no credentials and changes nothing:

```sh
plumb retire
```

Library consumers who enabled the `forge` Cargo feature enable `vendor`
instead, and reach the Forgejo client at `plumb::vendor::forgejo` rather than
`plumb::forge`. No published consumer enabled `forge`, so this is expected to
affect nothing outside Plumb itself.
