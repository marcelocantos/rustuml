# frozen_string_literal: true

require "json"
require "open3"

ROOT = File.expand_path(__dir__)
REPO = File.expand_path("../../../..", ROOT)
RUST = File.join(REPO, "target/release/rustuml")
JAVA_JAR = "/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar"

manifest = JSON.parse(File.read(File.join(ROOT, "cases.json")))
results = []

manifest.fetch("cases").each_with_index do |entry, index|
  source_path = File.join(ROOT, entry.fetch("source"))
  stem = source_path.delete_suffix(".puml")
  source = File.binread(source_path)

  java_stdout, java_stderr, java_status = Open3.capture3(
    "java", "-Djava.awt.headless=true", "-jar", JAVA_JAR, "-tsvg", "-pipe",
    stdin_data: source
  )
  File.binwrite("#{stem}.java.svg", java_stdout)
  File.binwrite("#{stem}.java.stderr", java_stderr)
  File.write("#{stem}.java.status", "#{java_status.exitstatus}\n")

  rust_stdout, rust_stderr, rust_status = Open3.capture3(
    RUST, "--no-oracle", "-tsvg", source_path
  )
  File.binwrite("#{stem}.rust.svg", rust_stdout)
  File.binwrite("#{stem}.rust.stderr", rust_stderr)
  File.write("#{stem}.rust.status", "#{rust_status.exitstatus}\n")

  yaml_stdout, yaml_stderr, yaml_status = Open3.capture3(
    RUST, "--no-oracle", "--yaml", source_path
  )
  File.binwrite("#{stem}.rust.yaml", yaml_stdout)
  File.binwrite("#{stem}.rust.yaml.stderr", yaml_stderr)
  File.write("#{stem}.rust.yaml.status", "#{yaml_status.exitstatus}\n")

  expected_valid = entry.fetch("expected_java_valid")
  java_accepted = java_status.success?
  rust_accepted = rust_status.success?
  result = if java_accepted != expected_valid
             "GENERATOR_EXPECTATION_MISMATCH"
           elsif java_accepted == rust_accepted
             java_accepted ? "ACCEPT_AGREEMENT" : "REJECT_AGREEMENT"
           elsif java_accepted
             "VALID_REJECTED_BY_RUST"
           else
             "MALFORMED_ACCEPTED_BY_RUST"
           end

  results << entry.merge(
    "java_exit" => java_status.exitstatus,
    "rust_exit" => rust_status.exitstatus,
    "rust_yaml_exit" => yaml_status.exitstatus,
    "result" => result
  )
  warn format("[%03d/%03d] %s: %s", index + 1, manifest.fetch("case_count"), entry.fetch("name"), result)
end

counts = results.group_by { |entry| entry.fetch("result") }.transform_values(&:length)
File.write(File.join(ROOT, "results.json"), JSON.pretty_generate({
  "schema_version" => 1,
  "rust_binary" => RUST,
  "java_jar" => JAVA_JAR,
  "commands" => {
    "java" => "java -Djava.awt.headless=true -jar #{JAVA_JAR} -tsvg -pipe < CASE.puml",
    "rust_svg" => "#{RUST} --no-oracle -tsvg CASE.puml",
    "rust_yaml" => "#{RUST} --no-oracle --yaml CASE.puml"
  },
  "counts" => counts,
  "results" => results
}) + "\n")

warn "result counts: #{counts.sort.to_h}"
