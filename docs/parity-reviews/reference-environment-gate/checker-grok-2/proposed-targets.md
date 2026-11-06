# Proposed targets

Not written to `bullseye.yaml`.

1. Pin every `tools/font-chain-reference/monkeyman-round9` `#slot` path that is outside `FONT_FILES`, the user-font directories, and the MobileAsset font walk. The 29 paths are `unpinned-cascade-slots.txt`. A change to the size or SHA256 of any of them must make `scripts/reference_env_census.py check` exit 1. This is the T14.6.8 gap: the owner pin is the system fonts the reference host actually opens, and the modelled-face list does not cover them.

2. Optional. Treat a scalar value of `-` the way a row hash of `-` is treated. Both sides `-` currently exits 0. Both sides blank already exits 2. `record` does not emit `-`, so this is not the reason the packet was rejected.
