# Monkey Man reference-environment census

Owner decision of 2026-11-06: Monkey Man's current fonts and Homebrew
HarfBuzz 15.0.0_1 / FreeType 2.15.0 are the Java oracle. Do not
`brew upgrade` on that host.

`census.txt` (sha256 `c0d8c8ab478d413d89cf04062c07e6ac9bcd8daede857e75d8453551db99da7c`)
is assembled from the stored relay artifacts, not from a fresh live hash.
`methodA.loaded.txt` is the round-6 `DYLD_PRINT_LIBRARIES` extract
(sha256 `794020d8d86bae63a9f573bb2a0806de1c6e2cc9d7e787b04e4cbd80eed0ad38`).

FreeType's loaded path is pinned. Its dylib sha256 is empty until the
live census in `tools/font-chain-reference/monkeyman-census-request/`
is committed over this file. User-font and MobileAsset rows pin path and
size; content hashes are `-` for the same reason. Modelled face files,
Times, Tahoma, Geeza Pro, HarfBuzz, the JDK `java` binary and
`libfontmanager.dylib` are hashed.

Regenerate with:

```
python3 -I scripts/reference_env_census.py assemble tools/font-chain-reference/monkeyman-reference-env/census.txt
```

The drift check is `reference_env_census.py check`. Exit 0 means the pinned
facts match. Exit 2 means they match and unpinned hashes are still open.
Exit 1 means the reference host moved.
