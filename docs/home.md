# Home

A tool that keeps state on the user's machine resolves one **data home**, and everything it writes
for that user lives under it. A tool that keeps no state resolves none — the home is not a ceremony
every binary must perform.

## Resolution

The data home is an ordinary cascade field. It needs no special door and no bootstrap exception:
`home` sits in the tool's config struct like `port` or `prefix`, so the four layers already answer
the question.

- **default** — the platform home: `$HOME/.<tool>` on unix, `%LOCALAPPDATA%\<tool>` on windows.
  Total, as every default layer must be.
- **file** — a repo-rooted config may point a project at its own home. Absent when the command runs
  outside a repo, which is the ordinary `resolve(None)` case, not an error.
- **environment** — `<TOOL>_HOME` falls out of the prefix derivation. No key is invented and no
  direct read of the environment is required.
- **arguments** — an explicit flag wins last.

A tool that reads `$HOME` itself has bypassed the cascade the same way an invented env key does.
Platform home resolution is `config`'s work, behind the one door.

## Layout

Under the home, `state/` holds machine-written records and nothing else. The rest of the home is the
tool's own vocabulary — plumb prescribes no further shape, the way it prescribes no config sections.

State files are machine-owned:

- **Schema-versioned.** A record carries the version of its own shape; a reader that meets an
  unknown version refuses rather than guesses.
- **Written whole.** A state write lands atomically — write beside, then replace — so an interrupted
  run leaves the previous record, never a torn one.
- **Not a config surface.** Hand editing is unsupported, not forbidden: the tool may rewrite the
  file at any time. Policy belongs in the cascade, where a human is the author; state belongs here,
  where the machine is.

## Ownership

State that records paths the tool created carries the burden of proving it created them. A record
alone is not proof — a path may be gone, replaced, or have become someone else's. Two-sided evidence
is the rule: the registry entry and a marker inside the path itself, both naming the tool. Acting on
a path that carries only one side is refused, and refusal is reported rather than repaired.

This is the same posture the config law takes toward a malformed file. A tool that deletes what it
did not create has done worse than one that stops.

The mechanized check — a doctor pass over a tool's declared home and its records — is declared here
and not yet running; until it lands, this page is the wall.
