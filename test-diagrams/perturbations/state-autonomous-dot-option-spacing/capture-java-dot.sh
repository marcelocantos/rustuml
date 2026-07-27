#!/bin/sh
set -eu

if [ "${1:-}" = "-V" ]; then
  exec /opt/homebrew/bin/dot "$@"
fi

: "${RUSTUML_DOT_CAPTURE_DIR:?set RUSTUML_DOT_CAPTURE_DIR}"
mkdir -p "$RUSTUML_DOT_CAPTURE_DIR"

index=1
while [ -e "$RUSTUML_DOT_CAPTURE_DIR/invocation-$(printf '%02d' "$index").dot" ]; do
  index=$((index + 1))
done

capture="$RUSTUML_DOT_CAPTURE_DIR/invocation-$(printf '%02d' "$index").dot"
tee "$capture" | /opt/homebrew/bin/dot "$@"
