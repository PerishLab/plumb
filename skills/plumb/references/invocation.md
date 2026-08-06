# plumb invocation

The complete command surface of the binary this brief describes. Read the
laws in `SKILL.md` first; this file is the index, not the reasoning.

```bash
plumb doctor [ROOT]           # shape report; nonzero when out of true or blind
plumb land [ROOT]             # land the current clean topic branch onto its base
plumb precommit [ROOT]        # prove exact committed changes stay inside --write prefixes
plumb radius --root [label=]path # which supplied roots resolve a product below --candidate
plumb rule list               # complete catalog; compose typed selectors
plumb rule show RULE_ID       # law, standing, evidence, owner, and tags
plumb rule namespaces         # registered namespace vocabulary
plumb rule tags               # registered tag vocabulary
plumb rule owners             # registered owner vocabulary
plumb lock [ROOT]             # print what a fresh affirmation would record
plumb changelog [ROOT]        # refuse a version whose en+zh changelog is absent or empty
plumb policy [ROOT] --write   # reconcile ectropy.toml against the repository
plumb release authority       # print the product's canonical public release authority
plumb release dispatch        # dispatch an exact or stable Forgejo release workflow
plumb release matrix          # derive the shared target matrix from plumb.toml
plumb release build           # build and archive one declared target
plumb release assemble        # gather targets and build declared attachments
plumb release source          # bind the frozen event branch, commit, channel, and version
plumb release compile         # seal one exact declared product artifact set
plumb release publish         # publish immutable objects and the exact seal
plumb release activate        # move stable consensus with its separate authority
plumb release inspect         # verify an exact seal or stable public surface
plumb release smoke           # exercise a generated manager on this platform
plumb release packport        # CI proof that stable is already an ancestor of main
plumb release recovery {arm,beta,stable} # exact one-time Plumb v0.18.14 recovery
plumb stable prepare          # create a writable operator-only stable release line
plumb stable pick             # append one cherry-pick -x candidate commit
plumb stable freeze           # make the stable release line immutable
plumb stable packport         # merge published stable into main and retain its line
plumb site plan               # derive the build, deploy, and readback plan
plumb site inspect            # read token, worker, and domain binding state
plumb site deploy             # build, deploy, and prove the public fingerprint
plumb skill install           # install this brief into detected agent directories
plumb skill stage             # unpack one exact candidate into a new explicit path
plumb skill status            # compare managed installs with stable, read-only
plumb skill upgrade --dry-run # print the exact stable movement without applying it
plumb skill upgrade           # move managed installs to the selected version
plumb skill list              # show managed installs
plumb skill uninstall         # remove only what is recorded as managed
```

Install, status, and upgrade admit stable only; `--version` pins an exact
stable release, while no version resolves the stable pointer. `--path` installs
to one explicit managed directory ending with `plumb`. `--force` replaces only
a seat proved by the ledger and marker. Upgrade leaves current seats untouched,
refuses an implicit rollback or same-version artifact drift, and treats an
explicit older stable version as deliberate rollback.

`stage` requires `--channel`, `--version`, and `--path`. Its channel must be
non-stable, its version exact, and its target absent and ending with `plumb`.
It writes a staged marker but never the managed ledger. The caller owns cleanup
of the surrounding isolated root.

The binary is the truth about its own flags: prefer `plumb <command> --help`
over assuming.
