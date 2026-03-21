// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::Serialize;

/// Default port for the PlantUML picoweb server.
pub const DEFAULT_PORT: u16 = 8787;

#[derive(Serialize)]
struct RenderRequest<'a> {
    source: &'a str,
    options: &'a [&'a str],
}

/// Returns the PlantUML server URL, respecting `PLANTUML_URL` env var.
/// Defaults to `http://127.0.0.1:8787`.
pub fn server_url() -> String {
    std::env::var("PLANTUML_URL").unwrap_or_else(|_| format!("http://127.0.0.1:{DEFAULT_PORT}"))
}

/// Renders PlantUML source to SVG via the picoweb server.
pub fn render_svg(input: &str) -> Result<String> {
    render(input, &["-tsvg"])
}

/// Renders with arbitrary options via the server's POST /render endpoint.
fn render(input: &str, options: &[&str]) -> Result<String> {
    let url = format!("{}/render", server_url());

    let body = serde_json::to_string(&RenderRequest {
        source: input,
        options,
    })
    .context("failed to serialize render request")?;

    let response = ureq::post(&url)
        .header("Content-Type", "application/json")
        .send(body.as_str())
        .with_context(|| {
            format!(
                "failed to connect to PlantUML server at {}. \
                 Start it with: java -jar plantuml.jar -picoweb:{DEFAULT_PORT}",
                server_url()
            )
        })?;

    let status = response.status();
    let text = response
        .into_body()
        .read_to_string()
        .context("failed to read response body")?;

    if status != 200 {
        bail!("PlantUML server returned {status}: {text}");
    }

    Ok(text)
}

/// Locates the PlantUML JAR file.
///
/// Checks `PLANTUML_JAR` env var first, then falls back to the standard
/// build location in the reference repo.
pub fn find_jar() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("PLANTUML_JAR") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
        bail!("PLANTUML_JAR={} does not exist", p.display());
    }

    let home = std::env::var("HOME").context("HOME not set")?;
    let libs_dir = PathBuf::from(home).join("work/github.com/plantuml/plantuml/build/libs");

    find_latest_jar(&libs_dir)
}

fn find_latest_jar(dir: &Path) -> Result<PathBuf> {
    let mut jars: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|ext| ext == "jar")
                && p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("plantuml-")
        })
        .collect();

    jars.sort();
    jars.pop()
        .with_context(|| format!("no plantuml-*.jar found in {}", dir.display()))
}

/// Runs PlantUML in pipe mode with the given arguments and returns stdout.
/// Used for preproc which isn't supported by picoweb.
fn run_pipe(jar: &Path, input: &str, extra_args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("java");
    cmd.arg("-jar")
        .arg(jar)
        .args(extra_args)
        .arg("-pipe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().context("failed to spawn java")?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().context("failed to open stdin")?;
        stdin
            .write_all(input.as_bytes())
            .context("failed to write to stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("failed to wait for java")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("PlantUML exited with {}: {}", output.status, stderr.trim());
    }

    String::from_utf8(output.stdout).context("PlantUML output is not valid UTF-8")
}

/// Runs PlantUML preprocessor and returns expanded source.
/// Uses pipe mode since picoweb doesn't support preproc.
pub fn run_preproc(jar: &Path, input: &str) -> Result<String> {
    run_pipe(jar, input, &["-preproc"])
}
