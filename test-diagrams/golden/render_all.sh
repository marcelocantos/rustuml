#!/bin/bash
# Batch render all .puml files through the PlantUML server
# Usage: ./render_all.sh [PLANTUML_URL]

set -euo pipefail

URL="${1:-http://localhost:8787/svg}"
DIR="$(cd "$(dirname "$0")" && pwd)"
JOBS=20  # parallel connections to server

echo "Rendering all .puml files under $DIR"
echo "Server: $URL"
echo "Parallelism: $JOBS"
echo ""

render_one() {
    local puml="$1"
    local svg="${puml%.puml}.svg"

    # Skip if SVG already exists and is newer than PUML
    if [[ -f "$svg" && "$svg" -nt "$puml" ]]; then
        echo "SKIP $puml"
        return 0
    fi

    local http_code
    http_code=$(curl -s -o "$svg" -w "%{http_code}" -X POST "$URL" \
        --data-binary "@$puml" \
        --max-time 30 2>/dev/null) || true

    if [[ "$http_code" == "200" && -s "$svg" ]]; then
        if grep -q "Syntax Error" "$svg" 2>/dev/null; then
            echo "ERR  $puml (syntax error in output)"
            return 0
        fi
        echo "OK   $puml"
        return 0
    else
        echo "FAIL $puml (HTTP $http_code)"
        rm -f "$svg"
        return 0
    fi
}

export -f render_one
export URL

TOTAL=$(find "$DIR" -name '*.puml' -type f | wc -l | tr -d ' ')
echo "Found $TOTAL .puml files"
echo ""

# Use GNU parallel for reliable parallel execution
find "$DIR" -name '*.puml' -type f | sort | parallel -j "$JOBS" render_one {} 2>&1 | tee /tmp/render_results.log

OK=$(grep -c '^OK ' /tmp/render_results.log 2>/dev/null || echo 0)
FAIL=$(grep -c '^FAIL\|^ERR' /tmp/render_results.log 2>/dev/null || echo 0)
SKIP=$(grep -c '^SKIP' /tmp/render_results.log 2>/dev/null || echo 0)

echo ""
echo "=== Results ==="
echo "Total: $TOTAL"
echo "OK:    $OK"
echo "Skip:  $SKIP"
echo "Fail:  $FAIL"
