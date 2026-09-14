# Restrained scenarios

## An operation is blind

Stop the stateful path and restore the named evidence. A network lookup,
manifest, lock, source tree, or policy that could not be read has not proved a
negative fact. Re-run the same operation after evidence is readable; do not
substitute a cached assumption.

## A release is being prepared

Use `plumb version --help`, `plumb release --help`, and `plumb ship --help` for
the exact command contracts. One ship run binds its source revision and release
identity, builds only declared artifacts, publishes immutable objects and an
exact seal, and reads the public bytes back. Depot is a separate marker consumer:
publish marker-exact configuration explicitly before ship when manager smoke
needs it, then publish the remaining mutable derivatives and advance stable
consensus explicitly after immutable readback. Neither command invokes the other.
Non-stable releases remain exact and have no moving pointer or activation.
An exact release binds `refs/tags/<exact-version>`, so push that tag before
dispatching one; its channel is read from the version, never named beside it.

Stable starts from an explicit release line:

```bash
plumb version prepare --version VERSION
plumb version pick --version VERSION --commit COMMIT
plumb version freeze --version VERSION
plumb release stamp --version VERSION
plumb depot configuration --marker VERSION --from /temporary/configuration
plumb ship dispatch --marker VERSION
plumb depot skill --marker VERSION --from /temporary/skill
plumb depot changelog --marker VERSION --from /temporary/changelog
plumb depot managers --marker VERSION
plumb depot channel --marker VERSION
plumb version rejoin --version VERSION
```

Inspect each subcommand before use. `prepare` also records the line's datum,
so the frozen candidate is judged against the registry answers that stood when
the line was cut; a line carrying no datum is out of true. Stable publication
requires the frozen line. `freeze` also derives the promotion source
and refuses unless exactly one published exact seal stands at the frozen
commit, so recover a failed exact release by rerunning it rather than by
tagging the next candidate. Rejoin settles ancestry after publication;
it does not rewrite a successful release result, and `prepare` and `freeze`
refuse while the last stable point still sits outside `origin/main`, so a late
rejoin is caught before the next line exists. Never delete the frozen line.

Before rejoin, integrate maintenance fixes into main through an ordinary proved
Member and Land. Preserve main's ongoing work and verify the resulting behavior;
a stable release may contain fixes that have never reached main. Only after real
content integration may version rejoin settle ancestry. Its unchanged main tree
proves no content recovery by itself. Keep the frozen line and historical markers.

## A skill candidate needs validation

Do not replace a managed stable seat. Stage the exact candidate under a new
isolated root, run the intended session against that path, and remove the
surrounding isolated root only after the caller has preserved any evidence it
needs. Promotion later uses the same source revision as the validated candidate.

## A worker reaches its stable domain

Keep the immutable Worker Version separate from its mutable Deployment. Ship
uploads and reads back the version while carrying its full version id. Depot
revalidates the same release marker before assigning that id 100% of traffic.
Never recover by rebuilding or by running a direct deploy from source.

## Observation is configured

Observation defaults muted. Do not enable it, choose a report path, or select
facts on an operator's behalf. When explicitly enabled, invalid observation
configuration refuses the audit record without replacing the command's own
result. Treat trace keys as opaque correlation values, never as authority or
lifecycle state.
