# Plumb v0.18.17

## Version-owned release artifacts

A release version may now carry an optional flat artifact directory at
`docs/CHANGELOG/v<base-version>/artifacts/`. Every direct regular file joins
the exact prerelease and stable seals under its filename. Plumb preserves the
bytes and proves their delivery without interpreting their purpose, type, or
contents.

The directory has no file-count limit. Generic release safety still refuses
links, nested entries, non-UTF-8 filenames, and collisions with derived release
artifacts.

Prereleases resolve the directory through their stable base version, so one
commit supplies identical version-owned bytes to candidate and permanent
release identities.
