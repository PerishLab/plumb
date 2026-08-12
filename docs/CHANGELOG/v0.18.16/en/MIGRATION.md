# Migrating to Plumb v0.18.16

Before upgrading, remove every active reference to the retired term from your
repository. Search tracked paths and tracked file contents, case-insensitively,
outside `docs/CHANGELOG`. Historical entries under that directory are exempt
and need no edit.

A declaration that your repository does not use the retired product is still a
reference and will be reported. Rewrite it to name the absent capability rather
than the absent product. Where the surrounding sentence already names the
category — a runtime model, a wrapper kind, a lifecycle responsibility — the
product name is redundant and can simply be dropped without weakening the
claim.

Do not attempt to satisfy the rule with an alternate spelling, a split string,
or a repository-local exclusion. The dictionary admits none of them, and the
term is matched without word boundaries.

If your repository keeps Doctor fixtures of its own, note that a governed tree
which is not a Git repository now reports blind on this rule. Initialise such
fixtures as repositories rather than treating the finding as a defect.
