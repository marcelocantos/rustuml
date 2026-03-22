#!/bin/sh
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
#
# Generate golden SVG files from the test matrix.
# Uses a Rust helper binary to produce all matrix test inputs,
# then renders each through Java PlantUML.
#
# Usage: scripts/generate-golden-matrix.sh [port]

set -e

PORT="${1:-8787}"
URL="http://127.0.0.1:${PORT}/render"
GOLDEN_DIR="test-fixtures/golden"

echo "Generating golden files from matrix (PlantUML at port ${PORT})..."

# Build and run the golden generator.
cargo run --quiet -p rustuml-oracle --example golden_gen -- "$URL" "$GOLDEN_DIR"
