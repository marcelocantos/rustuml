#!/usr/bin/env python3
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0

"""
Side-by-side viewer for goldens vs. rustuml output.

Walks test-diagrams/golden/ and serves a tree-organized browser. Click any
.puml to see the golden SVG (left) and the rustuml-rendered SVG (right).

Rendering is on-the-fly via the `diff_one` example binary; build it once with:

    cargo build --release --example diff_one -p rustuml-oracle

Then run:

    scripts/viewer.py

Open http://localhost:8765/ in a browser.
"""

import http.server
import json
import os
import socketserver
import subprocess
import sys
import urllib.parse
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GOLDEN_ROOT = REPO_ROOT / "test-diagrams" / "golden"
DIFF_ONE_BIN = REPO_ROOT / "target" / "release" / "examples" / "diff_one"
CHECK_ALL_BIN = REPO_ROOT / "target" / "release" / "examples" / "check_all"
PORT = 8788


def build_tree() -> dict:
    """Walk GOLDEN_ROOT and return a nested dict: {bucket: [puml_basename, ...]}."""
    tree = {}
    for bucket_dir in sorted(GOLDEN_ROOT.iterdir()):
        if not bucket_dir.is_dir():
            continue
        names = sorted(
            p.stem for p in bucket_dir.iterdir() if p.suffix == ".puml"
        )
        if names:
            tree[bucket_dir.name] = names
    return tree


def compute_diff_set() -> set:
    """Run check_all and return a set of 'bucket/name' strings whose
    rustuml render disagrees with the golden under the strict-XML
    comparator. Empty set when everything passes."""
    if not CHECK_ALL_BIN.is_file():
        sys.stderr.write(
            "[viewer] check_all binary missing; differences-only filter unavailable.\n"
            "         Build with: cargo build --release --example check_all -p rustuml-oracle\n"
        )
        return set()
    try:
        result = subprocess.run(
            [str(CHECK_ALL_BIN)],
            capture_output=True,
            cwd=str(REPO_ROOT),
            timeout=300,
        )
    except subprocess.TimeoutExpired:
        sys.stderr.write("[viewer] check_all timed out; differences filter empty.\n")
        return set()
    if result.returncode != 0:
        sys.stderr.write(f"[viewer] check_all failed: {result.stderr.decode()[:300]}\n")
        return set()
    try:
        items = json.loads(result.stdout.decode())
        return set(items)
    except json.JSONDecodeError as e:
        sys.stderr.write(f"[viewer] check_all output not JSON: {e}\n")
        return set()


INDEX_HTML = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>rustuml golden viewer</title>
<style>
  :root {
    color-scheme: light dark;
    --bg: #f7f7f8;
    --panel-bg: #ffffff;
    --border: #d0d0d4;
    --text: #1a1a1c;
    --muted: #6a6a72;
    --accent: #2563eb;
    --accent-bg: #e8f0ff;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #18181b;
      --panel-bg: #232327;
      --border: #38383e;
      --text: #e8e8ec;
      --muted: #888892;
      --accent: #60a5fa;
      --accent-bg: #1e2a44;
    }
  }
  html, body {
    margin: 0; padding: 0; height: 100%;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    background: var(--bg); color: var(--text);
  }
  #app { display: grid; grid-template-columns: 280px 1fr; height: 100vh; }
  #sidebar {
    border-right: 1px solid var(--border);
    overflow-y: auto;
    background: var(--panel-bg);
  }
  #sidebar header {
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
    font-size: 13px;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--muted);
    position: sticky; top: 0;
    background: var(--panel-bg);
  }
  #search {
    width: calc(100% - 28px);
    margin: 10px 14px 4px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }
  .toolbar {
    padding: 6px 14px 10px;
    border-bottom: 1px solid var(--border);
    display: flex; flex-direction: column; gap: 4px;
    font-size: 12px;
    color: var(--muted);
  }
  .toolbar label {
    display: flex; align-items: center; gap: 6px;
    cursor: pointer; user-select: none;
  }
  .empty-state {
    padding: 18px 14px;
    color: var(--muted);
    font-size: 13px;
    line-height: 1.5;
  }
  details { margin: 0; }
  details > summary {
    padding: 6px 14px;
    cursor: pointer;
    font-weight: 500;
    list-style: none;
  }
  details > summary::-webkit-details-marker { display: none; }
  details > summary::before {
    content: "▸ ";
    display: inline-block;
    width: 1em;
    color: var(--muted);
  }
  details[open] > summary::before { content: "▾ "; }
  ul { list-style: none; margin: 0; padding: 0 0 6px 0; }
  li a {
    display: block;
    padding: 3px 14px 3px 38px;
    color: var(--text);
    text-decoration: none;
    font-size: 13px;
    font-family: -apple-system, BlinkMacSystemFont, ui-monospace, monospace;
  }
  li a:hover { background: var(--accent-bg); }
  li a.active { background: var(--accent-bg); color: var(--accent); font-weight: 600; }
  #main { display: flex; flex-direction: column; overflow: hidden; }
  #header {
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--panel-bg);
    display: flex; align-items: center; gap: 16px;
  }
  #header h1 { margin: 0; font-size: 15px; font-weight: 600; }
  #header .puml-path { color: var(--muted); font-family: ui-monospace, monospace; font-size: 13px; }
  #panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    flex: 1; overflow: hidden;
  }
  .panel { display: flex; flex-direction: column; overflow: hidden; }
  .panel + .panel { border-left: 1px solid var(--border); }
  .panel-title {
    padding: 8px 16px;
    background: var(--panel-bg);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .panel-body {
    flex: 1;
    overflow: auto;
    background:
      repeating-conic-gradient(rgba(128,128,128,0.06) 0% 25%, transparent 0% 50%)
      50% / 16px 16px;
  }
  .panel-body object,
  .panel-body img { display: block; max-width: 100%; }
  #placeholder {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: var(--muted); font-size: 14px;
  }
  .hidden { display: none !important; }
</style>
</head>
<body>
<div id="app">
  <nav id="sidebar">
    <header>Goldens</header>
    <input id="search" type="search" placeholder="Filter…" autocomplete="off" />
    <div class="toolbar">
      <label>
        <input id="differences-only" type="checkbox" checked>
        Show only differences (<span id="diff-count">…</span>)
      </label>
      <span style="font-size: 11px;">↑↓ navigate · ←→ collapse/expand</span>
    </div>
    <div id="tree"></div>
  </nav>
  <main id="main">
    <div id="header" class="hidden">
      <h1 id="title"></h1>
      <span class="puml-path" id="path"></span>
    </div>
    <div id="panels" class="hidden">
      <div class="panel">
        <div class="panel-title">Golden (Java PlantUML)</div>
        <div class="panel-body"><object id="golden" type="image/svg+xml"></object></div>
      </div>
      <div class="panel">
        <div class="panel-title">rustuml</div>
        <div class="panel-body"><object id="rust" type="image/svg+xml"></object></div>
      </div>
    </div>
    <div id="placeholder">Select a golden from the tree.</div>
  </main>
</div>
<script>
  const tree = TREE_JSON;
  const diffSet = new Set(DIFFS_JSON);
  const treeEl = document.getElementById("tree");
  const searchEl = document.getElementById("search");
  const diffOnlyEl = document.getElementById("differences-only");
  const diffCountEl = document.getElementById("diff-count");
  const headerEl = document.getElementById("header");
  const panelsEl = document.getElementById("panels");
  const placeholderEl = document.getElementById("placeholder");
  const titleEl = document.getElementById("title");
  const pathEl = document.getElementById("path");
  const goldenEl = document.getElementById("golden");
  const rustEl = document.getElementById("rust");
  let activeLink = null;

  diffCountEl.textContent = diffSet.size;
  // When there's nothing to review, leave the filter off so the user
  // can still browse the corpus without unchecking the box first.
  if (diffSet.size === 0) {
    diffOnlyEl.checked = false;
    diffOnlyEl.disabled = true;
  }

  function renderTree(filter) {
    treeEl.innerHTML = "";
    const f = filter.toLowerCase();
    const diffOnly = diffOnlyEl.checked;
    let total = 0;
    for (const [bucket, names] of Object.entries(tree)) {
      let matches = names;
      if (diffOnly) {
        matches = matches.filter(n => diffSet.has(bucket + "/" + n));
      }
      if (f) {
        matches = matches.filter(
          n => n.toLowerCase().includes(f) || bucket.toLowerCase().includes(f)
        );
      }
      if (!matches.length) continue;
      total += matches.length;
      const details = document.createElement("details");
      details.open = f.length > 0 || diffOnly;
      const summary = document.createElement("summary");
      summary.textContent = bucket + " (" + matches.length + ")";
      details.appendChild(summary);
      const ul = document.createElement("ul");
      for (const name of matches) {
        const li = document.createElement("li");
        const a = document.createElement("a");
        a.href = "#" + encodeURIComponent(bucket + "/" + name);
        a.textContent = name;
        a.dataset.bucket = bucket;
        a.dataset.name = name;
        a.tabIndex = -1;
        a.addEventListener("click", e => {
          e.preventDefault();
          window.location.hash = a.getAttribute("href").slice(1);
        });
        li.appendChild(a);
        ul.appendChild(li);
      }
      details.appendChild(ul);
      treeEl.appendChild(details);
    }
    if (total === 0) {
      const empty = document.createElement("div");
      empty.className = "empty-state";
      if (diffOnly && diffSet.size === 0) {
        empty.innerHTML =
          "Nothing to review — every golden matches the rustuml render " +
          "under the strict‑XML comparator. " +
          "<br><br>Uncheck “Show only differences” to browse " +
          "the whole corpus.";
      } else {
        empty.textContent = "No matches.";
      }
      treeEl.appendChild(empty);
    }
  }

  function show(bucket, name) {
    titleEl.textContent = bucket + " / " + name;
    pathEl.textContent = "test-diagrams/golden/" + bucket + "/" + name + ".puml";
    const path = bucket + "/" + name;
    goldenEl.setAttribute("data", "/golden/" + path + ".svg");
    rustEl.setAttribute("data", "/render/" + path + ".puml");
    headerEl.classList.remove("hidden");
    panelsEl.classList.remove("hidden");
    placeholderEl.classList.add("hidden");
    if (activeLink) activeLink.classList.remove("active");
    activeLink = treeEl.querySelector(
      `a[data-bucket="${CSS.escape(bucket)}"][data-name="${CSS.escape(name)}"]`
    );
    if (activeLink) {
      activeLink.classList.add("active");
      activeLink.closest("details").open = true;
      activeLink.scrollIntoView({ block: "nearest" });
    }
  }

  function syncFromHash() {
    const h = decodeURIComponent(location.hash.replace(/^#/, ""));
    if (!h) return;
    const ix = h.indexOf("/");
    if (ix < 0) return;
    show(h.slice(0, ix), h.slice(ix + 1));
  }

  // List of leaf anchors in document order (open groups only — collapsed
  // groups are skipped, matching what's visible). Rebuilt on demand.
  function visibleLinks() {
    return Array.from(
      treeEl.querySelectorAll("details[open] > ul > li > a")
    );
  }

  function activeOrFirst() {
    if (activeLink && activeLink.isConnected && activeLink.offsetParent !== null) {
      return activeLink;
    }
    const links = visibleLinks();
    return links.length ? links[0] : null;
  }

  function navigate(delta) {
    const links = visibleLinks();
    if (!links.length) return;
    const cur = activeOrFirst();
    const ix = cur ? links.indexOf(cur) : -1;
    const next = links[Math.max(0, Math.min(links.length - 1, ix + delta))];
    if (next && next !== cur) {
      window.location.hash = next.getAttribute("href").slice(1);
    }
  }

  function detailsOf(link) {
    return link ? link.closest("details") : null;
  }

  function bucketDetailsList() {
    return Array.from(treeEl.querySelectorAll(":scope > details"));
  }

  function activeOrFocusedDetails() {
    if (activeLink) return detailsOf(activeLink);
    const all = bucketDetailsList();
    return all.length ? all[0] : null;
  }

  function expandActiveBucket() {
    const d = activeOrFocusedDetails();
    if (!d) return;
    if (!d.open) { d.open = true; renderActiveHighlight(); }
  }

  function collapseActiveBucket() {
    const d = activeOrFocusedDetails();
    if (!d) return;
    if (d.open) {
      d.open = false;
      // If the active leaf was inside this group, it's no longer visible.
      // Activate the bucket's summary by selecting its first child.
      activeLink = null;
      renderActiveHighlight();
    }
  }

  function renderActiveHighlight() {
    treeEl.querySelectorAll("a.active").forEach(a => a.classList.remove("active"));
    if (activeLink && activeLink.isConnected) {
      activeLink.classList.add("active");
    }
  }

  document.addEventListener("keydown", e => {
    // Ignore keystrokes while typing in the filter / search.
    if (e.target instanceof HTMLInputElement) return;
    switch (e.key) {
      case "ArrowDown": e.preventDefault(); navigate(1); break;
      case "ArrowUp":   e.preventDefault(); navigate(-1); break;
      case "ArrowRight": e.preventDefault(); expandActiveBucket(); break;
      case "ArrowLeft":  e.preventDefault(); collapseActiveBucket(); break;
      case "/":         e.preventDefault(); searchEl.focus(); break;
    }
  });

  searchEl.addEventListener("input", () => renderTree(searchEl.value));
  diffOnlyEl.addEventListener("change", () => renderTree(searchEl.value));
  window.addEventListener("hashchange", syncFromHash);
  renderTree("");
  syncFromHash();
</script>
</body>
</html>
"""


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Quieter logs; just method + path
        sys.stderr.write("[viewer] %s %s\n" % (self.command, self.path))

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        try:
            if path == "/" or path == "/index.html":
                self.send_index()
            elif path.startswith("/golden/"):
                self.send_golden(path[len("/golden/"):])
            elif path.startswith("/render/"):
                self.send_render(path[len("/render/"):])
            else:
                self.send_error(404, "Not found")
        except BrokenPipeError:
            pass

    def send_index(self):
        tree = build_tree()
        diff_set = self.server.diff_set  # populated at startup
        body = (
            INDEX_HTML
            .replace("TREE_JSON", json.dumps(tree))
            .replace("DIFFS_JSON", json.dumps(sorted(diff_set)))
        ).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def send_golden(self, rel):
        full = (GOLDEN_ROOT / rel).resolve()
        if not str(full).startswith(str(GOLDEN_ROOT)):
            self.send_error(403, "Forbidden")
            return
        if not full.is_file():
            self.send_error(404, "Golden not found: " + rel)
            return
        data = full.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", "image/svg+xml")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def send_render(self, rel):
        full = (GOLDEN_ROOT / rel).resolve()
        if not str(full).startswith(str(GOLDEN_ROOT)):
            self.send_error(403, "Forbidden")
            return
        if not full.is_file():
            self.send_error(404, "Puml not found: " + rel)
            return
        if not DIFF_ONE_BIN.is_file():
            msg = (
                "diff_one binary missing. Run:\n"
                "  cargo build --release --example diff_one -p rustuml-oracle\n"
            ).encode("utf-8")
            self.send_response(500)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", str(len(msg)))
            self.end_headers()
            self.wfile.write(msg)
            return
        try:
            result = subprocess.run(
                [str(DIFF_ONE_BIN), str(full), "--print-rust"],
                capture_output=True,
                cwd=str(REPO_ROOT),
                timeout=30,
                env={**os.environ, "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100"},
            )
        except subprocess.TimeoutExpired:
            self.send_error(504, "Render timed out")
            return
        if result.returncode != 0:
            err = (b"<!-- render failed: -->\n" + result.stderr)
            self.send_response(200)
            self.send_header("Content-Type", "image/svg+xml")
            self.send_header("Content-Length", str(len(err)))
            self.end_headers()
            self.wfile.write(err)
            return
        body = result.stdout
        self.send_response(200)
        self.send_header("Content-Type", "image/svg+xml")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main():
    if not GOLDEN_ROOT.is_dir():
        print(f"error: golden directory not found at {GOLDEN_ROOT}", file=sys.stderr)
        sys.exit(1)
    if not DIFF_ONE_BIN.is_file():
        print(
            "warning: diff_one binary missing. Build it with:\n"
            "  cargo build --release --example diff_one -p rustuml-oracle\n"
            "(starting server anyway; renders will fail until you build)",
            file=sys.stderr,
        )
    print("computing strict-XML differences …", end=" ", flush=True)
    diff_set = compute_diff_set()
    print(f"{len(diff_set)} differing")
    server = socketserver.ThreadingTCPServer(("127.0.0.1", PORT), Handler)
    server.allow_reuse_address = True
    server.diff_set = diff_set
    print(f"viewer running at http://127.0.0.1:{PORT}/")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nshutting down")
        server.shutdown()


if __name__ == "__main__":
    main()
