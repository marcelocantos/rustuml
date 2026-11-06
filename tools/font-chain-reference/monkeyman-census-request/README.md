# Monkey Man reference-environment census

Read-only. The parent runs this on Monkey Man. The script writes only under
the empty output directory it is given. It does not install packages, change
Homebrew, or write anywhere else.

Copy this directory to Monkey Man unchanged. `reference_env_census.py` and
`census.txt` are the checkout's `scripts/reference_env_census.py` and
`tools/font-chain-reference/monkeyman-reference-env/census.txt`.

```sh
OUT=<empty directory>
python3 -I reference_env_census.py record "$OUT"
python3 -I reference_env_census.py check "$OUT" --committed census.txt
```

`record` runs the reference JVM once with `DYLD_PRINT_LIBRARIES` and hashes
the loaded HarfBuzz and FreeType libraries, the JDK, the modelled face files,
`~/Library/Fonts`, `/Library/Fonts`, and the MobileAsset font trees. It hashes
PingFang and LiHei Pro in full and records the size of every other asset font.

`check` exits 0 when every pinned fact matches the committed census, 1 when a
pinned path, size or hash moved, and 2 when the pinned facts match but hashes
the baseline still leaves open (FreeType's dylib, user-font contents, most
MobileAsset contents) are filled only on the live side. Exit 2 is not drift.
Return `$OUT/census.txt` so it can be committed as the live baseline. Do not
`brew upgrade` on this host.
