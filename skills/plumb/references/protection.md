# Repository protection

This law is prose-only. `plumb doctor` does not read the Forgejo control plane.
The operator applies it with a local credential, reads the result back, and
refuses drift.

Every repository carrying a root `plumb.toml` has the same two baseline rules:

- `main` byte-matches `main-protection.bytes`;
- `release/**` byte-matches `release-protection.bytes`.

There are no repository-specific fields, status contexts, approval counts,
users, teams, or exceptions. The `main` rule blocks direct push for admins too
and requires the common `guard / guard (pull_request)` context. Land through
`plumb land`, whose aggregate status wait also keeps repository-specific
platform jobs effective without putting them into branch policy.

`release/**` is the fail-closed floor. Every existing `release/vX.Y.Z` branch
also has one exact rule, because Forgejo gives an exact rule precedence over a
glob:

- PREPARING renders `release-preparing-protection.bytes` by replacing both
  occurrences of `release/vX.Y.Z` with the exact branch name. Only the
  canonical local operator `PerishFire` may push, and every change is a linear
  `cherry-pick -x`.
- FROZEN, ACTIVATED, and SETTLED render
  `release-frozen-protection.bytes` with the same substitution. No push is
  admitted.

A release branch is permanent. Packport changes ancestry in `main`; it never
removes the exact protection or deletes the branch. If historical stable
consensus names a commit but its release branch is absent, recreate the branch
at that exact commit and apply the FROZEN document.

## Byte comparison

Read each rule through the Forgejo API, reject every rule outside `main`,
`release/**`, and exact `release/vX.Y.Z` branches that actually exist, then
project these keys in this order:

```text
branch_name rule_name enable_push enable_push_whitelist
push_whitelist_usernames push_whitelist_teams push_whitelist_deploy_keys
enable_merge_whitelist merge_whitelist_usernames merge_whitelist_teams
enable_status_check status_check_contexts required_approvals
enable_approvals_whitelist approvals_whitelist_username
approvals_whitelist_teams block_on_rejected_reviews
block_on_official_review_requests block_on_outdated_branch
dismiss_stale_approvals ignore_stale_approvals require_signed_commits
protected_file_patterns unprotected_file_patterns apply_to_admins
```

Serialize the projection as two-space JSON with one trailing newline and
compare bytes with the corresponding document. `created_at` and `updated_at`
are server observations and are the only omitted response fields. Do not sort,
coerce, default, or otherwise normalize a value. Forgejo reports an empty
`branch_name` for the `release/**` glob; the canonical bytes retain that empty
server value while `rule_name` carries the required glob.
