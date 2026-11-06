# Proposed targets

Not written to `bullseye.yaml`. The checker does not edit that file.

1. User-font symlink rows pin the target path and a hash of the target file. `check` exits 1 when either changes, including a same-length retarget and an in-place edit of `/Library/Fonts/Arial Unicode.ttf`. The system file that link points at is either in the hashed system set or the 22-file list is declared incomplete against T14.6.8.

2. `seq_capture.py` on Monkey Man runs `python3 -I reference_env_census.py record` and `check` against the committed census before `new-worker` and before any reference invocation. The batch aborts on exit 1 and on exit 2. Exit 2 is not success.

3. `record` does not create missing ancestors of OUT. The only directory it may create is OUT itself, and only when that directory is absent or empty. A test locks that, including the `DYLD_PRINT_LIBRARIES` child environment, without running Java on the origin Mac.
