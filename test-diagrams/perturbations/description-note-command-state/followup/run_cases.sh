#!/bin/sh
set -u

ROOT=/private/tmp/rustuml-description-note-checker
OUT="$ROOT/test-diagrams/perturbations/description-note-command-state/followup"
RUSTUML=/private/tmp/rustuml-note-terminal-followup2/target/release/rustuml
PLANTUML_JAR=/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar

{
    printf 'rust_revision='
    git -C "$ROOT" rev-parse HEAD
    printf 'rust_binary='
    shasum -a 256 "$RUSTUML"
    printf 'java_revision='
    git -C /Users/marcelo/work/github.com/plantuml/plantuml rev-parse HEAD
    printf 'java_jar='
    shasum -a 256 "$PLANTUML_JAR"
    "$RUSTUML" --version
    java -version 2>&1
    rsvg-convert --version
} >"$OUT/environment.txt"

for source in "$OUT"/*.puml; do
    stem=${source%.puml}

    if java -Djava.awt.headless=true -jar "$PLANTUML_JAR" -pipe -tsvg \
        <"$source" >"$stem.java.svg" 2>"$stem.java.stderr"; then
        java_status=0
    else
        java_status=$?
    fi
    printf '%s\n' "$java_status" >"$stem.java.status"

    if "$RUSTUML" -pipe -tsvg \
        <"$source" >"$stem.rust.svg" 2>"$stem.rust.stderr"; then
        rust_status=0
    else
        rust_status=$?
    fi
    printf '%s\n' "$rust_status" >"$stem.rust.status"

    if "$RUSTUML" --yaml "$source" \
        >"$stem.rust.yaml" 2>"$stem.rust-yaml.stderr"; then
        rust_yaml_status=0
    else
        rust_yaml_status=$?
    fi
    printf '%s\n' "$rust_yaml_status" >"$stem.rust-yaml.status"

    if rsvg-convert "$stem.java.svg" -o "$stem.java.png" \
        2>"$stem.java-raster.stderr"; then
        java_raster_status=0
    else
        java_raster_status=$?
    fi
    printf '%s\n' "$java_raster_status" >"$stem.java-raster.status"

    if rsvg-convert "$stem.rust.svg" -o "$stem.rust.png" \
        2>"$stem.rust-raster.stderr"; then
        rust_raster_status=0
    else
        rust_raster_status=$?
    fi
    printf '%s\n' "$rust_raster_status" >"$stem.rust-raster.status"
done
