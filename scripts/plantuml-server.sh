#!/bin/sh
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
#
# Starts the PlantUML picoweb server for oracle tests.
# Usage: scripts/plantuml-server.sh [port]

set -e

PORT="${1:-8787}"

JAR="${PLANTUML_JAR:-}"
if [ -z "$JAR" ]; then
    JAR=$(ls -1 ~/work/github.com/plantuml/plantuml/build/libs/plantuml-*.jar 2>/dev/null | sort | tail -1)
fi

if [ -z "$JAR" ] || [ ! -f "$JAR" ]; then
    echo "PlantUML JAR not found." >&2
    echo "Build it: cd ~/work/github.com/plantuml/plantuml && ./gradlew build -Pfast -x test" >&2
    echo "Or set PLANTUML_JAR=/path/to/plantuml.jar" >&2
    exit 1
fi

echo "Starting PlantUML server on port $PORT (JAR: $JAR)"
exec java -jar "$JAR" "-picoweb:$PORT"
