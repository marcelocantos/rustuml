// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Test-only access to PlantUML's dormant `-debugsvek` checkpoint files.
//!
//! The pinned source class is copied to a temporary directory, amended at its
//! existing `BaseFile` seam, compiled against the pinned JAR, and placed ahead
//! of that JAR on the classpath. PlantUML's source tree and JAR stay unchanged.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use wait_timeout::ChildExt;

use crate::runner;

const SVEK_MAKER_RELATIVE_PATH: &str =
    "src/main/java/net/sourceforge/plantuml/svek/CucaDiagramFileMakerSvek.java";
const PINNED_PLANTUML_REVISION: &str = "71806a23780b04a5ccde2f8ceb5121edad5eb711";
const PINNED_PLANTUML_JAR: &str = "plantuml-1.2026.3beta6.jar";
const PINNED_PLANTUML_JAR_SHA256: &str =
    "c4d7d78bf9bc1145299d02f648d7895d4056dc262a091d25c91714dec0478d45";
const ORIGINAL_BASEFILE_BLOCK: &str = "\t\tBaseFile basefile = null;\n\
//\t\tif (fileFormatOption.isDebugSvek() && os instanceof NamedOutputStream)\n\
//\t\t\tbasefile = ((NamedOutputStream) os).getBasefile();";
const TRACED_BASEFILE_BLOCK: &str = "\t\tBaseFile basefile = null;\n\
\t\tfinal String rustumlTraceBase = System.getProperty(\"rustuml.trace.base\");\n\
\t\tif (fileFormatOption.isDebugSvek() && rustumlTraceBase != null)\n\
\t\t\tbasefile = new BaseFile(new net.sourceforge.plantuml.security.SFile(rustumlTraceBase));";

#[derive(Debug)]
pub struct JavaSvekTrace {
    pub revision: String,
    pub jar_sha256: String,
    pub rendered_svg: String,
    pub request_dot: String,
    pub solved_svg: String,
}

pub struct JavaSvekTracer {
    _workspace: TemporaryWorkspace,
    classes: PathBuf,
    jar: PathBuf,
    revision: String,
    jar_sha256: String,
}

struct TemporaryWorkspace {
    path: PathBuf,
}

impl TemporaryWorkspace {
    fn create() -> Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock predates Unix epoch")?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rustuml-parity-trace-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .with_context(|| format!("cannot create {}", path.display()))?;
        Ok(Self { path })
    }
}

impl Drop for TemporaryWorkspace {
    fn drop(&mut self) {
        let owned = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rustuml-parity-trace-"));
        if owned && self.path.starts_with(std::env::temp_dir()) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

impl JavaSvekTracer {
    pub fn new() -> Result<Self> {
        let jar = runner::find_jar()?;
        ensure!(
            jar.file_name().and_then(|name| name.to_str()) == Some(PINNED_PLANTUML_JAR),
            "parity traces require {PINNED_PLANTUML_JAR}, found {}",
            jar.display()
        );
        let jar_sha256 = file_sha256(&jar)?;
        ensure!(
            jar_sha256 == PINNED_PLANTUML_JAR_SHA256,
            "parity traces require PlantUML JAR SHA-256 {PINNED_PLANTUML_JAR_SHA256}, found {jar_sha256}"
        );
        let source_root = plantuml_source_root()?;
        let revision = git_revision(&source_root)?;
        ensure!(
            revision == PINNED_PLANTUML_REVISION,
            "parity traces require PlantUML {PINNED_PLANTUML_REVISION}, found {revision}"
        );
        let workspace = TemporaryWorkspace::create()?;
        let classes = workspace.path.join("classes");
        let overlay_source = workspace
            .path
            .join("src/net/sourceforge/plantuml/svek/CucaDiagramFileMakerSvek.java");
        std::fs::create_dir_all(
            overlay_source
                .parent()
                .context("overlay source has no parent")?,
        )?;
        std::fs::create_dir_all(&classes)?;

        let upstream_source = git_file_at_revision(
            &source_root,
            PINNED_PLANTUML_REVISION,
            SVEK_MAKER_RELATIVE_PATH,
        )?;
        ensure!(
            upstream_source.matches(ORIGINAL_BASEFILE_BLOCK).count() == 1,
            "pinned PlantUML SVEK BaseFile seam changed at {SVEK_MAKER_RELATIVE_PATH}"
        );
        let overlay = upstream_source.replacen(ORIGINAL_BASEFILE_BLOCK, TRACED_BASEFILE_BLOCK, 1);
        std::fs::write(&overlay_source, overlay)
            .with_context(|| format!("cannot write {}", overlay_source.display()))?;

        let javac = command_output(
            Command::new("javac")
                .arg("-cp")
                .arg(&jar)
                .arg("-d")
                .arg(&classes)
                .arg(&overlay_source),
            "PlantUML trace overlay compilation",
        )?;
        if !javac.status.success() {
            bail!(
                "PlantUML trace overlay compilation failed: {}",
                String::from_utf8_lossy(&javac.stderr).trim()
            );
        }

        Ok(Self {
            _workspace: workspace,
            classes,
            jar,
            revision,
            jar_sha256,
        })
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn jar_path(&self) -> &Path {
        &self.jar
    }

    pub fn jar_sha256(&self) -> &str {
        &self.jar_sha256
    }

    pub fn capture(&self, source: &str, output_dir: &Path) -> Result<JavaSvekTrace> {
        self.capture_candidate(source, output_dir)?
            .context("PlantUML rejected the traced diagram")
    }

    pub fn capture_candidate(
        &self,
        source: &str,
        output_dir: &Path,
    ) -> Result<Option<JavaSvekTrace>> {
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("cannot create {}", output_dir.display()))?;
        let input = output_dir.join("source.puml");
        let rendered_svg_path = input.with_extension("svg");
        let request_dot_path = output_dir.join("java_svek.dot");
        let solved_svg_path = output_dir.join("java_svek.svg");
        for artifact in [&rendered_svg_path, &request_dot_path, &solved_svg_path] {
            match std::fs::remove_file(artifact) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).context(format!("cannot remove {}", artifact.display()));
                }
            }
        }
        std::fs::write(&input, source)
            .with_context(|| format!("cannot write {}", input.display()))?;
        let trace_base = output_dir.join("java.svg");
        let classpath = std::env::join_paths([self.classes.as_path(), self.jar.as_path()])?;
        let java = command_output(
            Command::new("java")
                .arg("-Djava.awt.headless=true")
                .arg(format!(
                    "-Drustuml.trace.base={}",
                    trace_base.to_string_lossy()
                ))
                .arg("-cp")
                .arg(classpath)
                .arg("net.sourceforge.plantuml.Run")
                .arg("-debugsvek")
                .arg("-tsvg")
                .arg(&input),
            "PlantUML traced render",
        )?;
        if !java.status.success() {
            if java.status.code() == Some(200) {
                return Ok(None);
            }
            bail!(
                "PlantUML trace run failed with {}: {}",
                java.status,
                String::from_utf8_lossy(&java.stderr).trim()
            );
        }

        let rendered_svg = read_required(&rendered_svg_path)?;
        let (Some(request_dot), Some(solved_svg)) = (
            read_optional(&request_dot_path)?,
            read_optional(&solved_svg_path)?,
        ) else {
            return Ok(None);
        };
        Ok(Some(JavaSvekTrace {
            revision: self.revision.clone(),
            jar_sha256: self.jar_sha256.clone(),
            rendered_svg,
            request_dot,
            solved_svg,
        }))
    }
}

pub fn capture_java_svek(source: &str, output_dir: &Path) -> Result<JavaSvekTrace> {
    JavaSvekTracer::new()?.capture(source, output_dir)
}

fn plantuml_source_root() -> Result<PathBuf> {
    if let Some(root) = std::env::var_os("PLANTUML_SOURCE_ROOT") {
        let root = PathBuf::from(root);
        ensure!(root.join(SVEK_MAKER_RELATIVE_PATH).is_file());
        return Ok(root);
    }
    let home = std::env::var_os("HOME").context("HOME not set")?;
    let root = PathBuf::from(home).join("work/github.com/plantuml/plantuml");
    ensure!(
        root.join(SVEK_MAKER_RELATIVE_PATH).is_file(),
        "PlantUML source not found under {}; set PLANTUML_SOURCE_ROOT",
        root.display()
    );
    Ok(root)
}

fn git_revision(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to read the PlantUML revision")?;
    ensure!(output.status.success(), "git rev-parse failed for PlantUML");
    String::from_utf8(output.stdout)
        .context("PlantUML revision is not UTF-8")
        .map(|revision| revision.trim().to_string())
}

fn git_file_at_revision(root: &Path, revision: &str, path: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{revision}:{path}")])
        .output()
        .context("failed to read pinned PlantUML source")?;
    ensure!(
        output.status.success(),
        "git show failed for PlantUML {revision}:{path}"
    );
    String::from_utf8(output.stdout).context("PlantUML source is not UTF-8")
}

fn read_required(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("missing trace artifact {}", path.display()))
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("cannot read {}", path.display())),
    }
}

fn file_sha256(path: &Path) -> Result<String> {
    let output = command_output(
        Command::new("shasum").args(["-a", "256"]).arg(path),
        "PlantUML JAR hashing",
    )?;
    ensure!(
        output.status.success(),
        "shasum failed for {}",
        path.display()
    );
    String::from_utf8(output.stdout)
        .context("shasum output is not UTF-8")?
        .split_whitespace()
        .next()
        .context("shasum returned no digest")
        .map(str::to_string)
}

fn command_output(command: &mut Command, label: &str) -> Result<Output> {
    command_output_with_timeout(command, label, Duration::from_secs(30))
}

fn command_output_with_timeout(
    command: &mut Command,
    label: &str,
    timeout: Duration,
) -> Result<Output> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {label}"))?;
    let mut stdout = child.stdout.take().context("child stdout missing")?;
    let mut stderr = child.stderr.take().context("child stderr missing")?;
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let Some(status) = child
        .wait_timeout(timeout)
        .with_context(|| format!("failed while waiting for {label}"))?
    else {
        #[cfg(unix)]
        // SAFETY: the child was started as leader of its own process group;
        // the negative-scope kill cannot target the parent process group.
        unsafe {
            libc::killpg(child.id() as libc::pid_t, libc::SIGKILL);
        }
        #[cfg(not(unix))]
        let _ = child.kill();
        let _ = child.wait();
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        bail!("{label} timed out after {} ms", timeout.as_millis());
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| anyhow::anyhow!("{label} stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| anyhow::anyhow!("{label} stderr reader panicked"))??;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Duration;

    use super::{command_output, command_output_with_timeout};

    #[test]
    fn command_output_drains_large_stdout_and_stderr() {
        let executable = std::env::current_exe().unwrap();
        let output = command_output(
            Command::new(executable).args([
                "--ignored",
                "--exact",
                "java_trace::tests::emit_large_child_output",
                "--nocapture",
            ]),
            "large-output test child",
        )
        .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.len() + output.stderr.len() > 400_000);
    }

    #[test]
    fn command_output_bounds_a_stuck_process_group() {
        let executable = std::env::current_exe().unwrap();
        let error = command_output_with_timeout(
            Command::new(executable).args([
                "--ignored",
                "--exact",
                "java_trace::tests::park_child",
                "--nocapture",
            ]),
            "stuck test child",
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert!(error.to_string().contains("timed out"));
    }

    #[test]
    #[ignore]
    fn emit_large_child_output() {
        print!("{}", "o".repeat(256_000));
        eprint!("{}", "e".repeat(256_000));
    }

    #[test]
    #[ignore]
    fn park_child() {
        std::thread::park_timeout(Duration::from_secs(10));
    }
}
