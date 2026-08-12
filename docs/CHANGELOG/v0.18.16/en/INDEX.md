# Plumb v0.18.16

## The retired dictionary carries its first term

The transitional retired dictionary shipped its mechanism with an empty set, so
`vocabulary.retired-term-absent` never had a subject and the scan returned
before reading any repository. The dictionary now carries one encoded atom, and
the rule is live for every governed repository.

A governed repository is out of true when the term appears in a tracked path or
in the current working-tree bytes of a tracked file. `docs/CHANGELOG` is the
sole exemption. Matching is case-insensitive and has no word boundary, so a
sentence declaring that the repository does not use the retired product still
carries the term and is a finding like any other.

## Doctor fixtures are repositories

Enabling the dictionary exercised the vocabulary path for the first time and
found that the test fixtures modelled a governed repository that is not a Git
repository. Doctor reported those fixtures blind, correctly: its evidence is
tracked paths, which a non-repository cannot supply. Every fixture that runs
Doctor is now a repository. The rule itself is unchanged.
